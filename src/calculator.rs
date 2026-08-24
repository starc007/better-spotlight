const MAX_ABS_RESULT: f64 = 1.0e100;

pub fn evaluate(expression: &str) -> Option<String> {
    let expression = expression.trim();
    if expression.is_empty() || !expression.bytes().any(|byte| byte.is_ascii_digit()) {
        return None;
    }

    let mut parser = Parser::new(expression);
    let value = parser.parse_expression()?;
    parser.skip_whitespace();
    if parser.position != parser.input.len() || !value.is_finite() || value.abs() > MAX_ABS_RESULT {
        return None;
    }
    Some(format_result(value))
}

fn format_result(value: f64) -> String {
    let value = if value == -0.0 { 0.0 } else { value };
    if value.fract() == 0.0 && value.abs() < 1.0e15 {
        return format!("{value:.0}");
    }

    let mut formatted = format!("{value:.10}");
    while formatted.ends_with('0') {
        formatted.pop();
    }
    if formatted.ends_with('.') {
        formatted.pop();
    }
    formatted
}

struct Parser<'a> {
    input: &'a [u8],
    position: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            position: 0,
        }
    }

    fn parse_expression(&mut self) -> Option<f64> {
        let mut value = self.parse_term()?;
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some(b'+') => {
                    self.position += 1;
                    value += self.parse_term()?;
                }
                Some(b'-') => {
                    self.position += 1;
                    value -= self.parse_term()?;
                }
                _ => return Some(value),
            }
        }
    }

    fn parse_term(&mut self) -> Option<f64> {
        let mut value = self.parse_power()?;
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some(b'*') => {
                    self.position += 1;
                    value *= self.parse_power()?;
                }
                Some(b'/') => {
                    self.position += 1;
                    let divisor = self.parse_power()?;
                    if divisor == 0.0 {
                        return None;
                    }
                    value /= divisor;
                }
                Some(b'%') => {
                    self.position += 1;
                    let divisor = self.parse_power()?;
                    if divisor == 0.0 {
                        return None;
                    }
                    value %= divisor;
                }
                _ => return Some(value),
            }
        }
    }

    fn parse_power(&mut self) -> Option<f64> {
        let value = self.parse_unary()?;
        self.skip_whitespace();
        if self.peek() == Some(b'^') {
            self.position += 1;
            Some(value.powf(self.parse_power()?))
        } else {
            Some(value)
        }
    }

    fn parse_unary(&mut self) -> Option<f64> {
        self.skip_whitespace();
        match self.peek() {
            Some(b'+') => {
                self.position += 1;
                self.parse_unary()
            }
            Some(b'-') => {
                self.position += 1;
                self.parse_unary().map(|value| -value)
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Option<f64> {
        self.skip_whitespace();
        if self.peek() == Some(b'(') {
            self.position += 1;
            let value = self.parse_expression()?;
            self.skip_whitespace();
            (self.peek() == Some(b')')).then(|| self.position += 1)?;
            return Some(value);
        }
        self.parse_number()
    }

    fn parse_number(&mut self) -> Option<f64> {
        self.skip_whitespace();
        let start = self.position;
        let mut decimal_seen = false;
        while let Some(byte) = self.peek() {
            if byte.is_ascii_digit() {
                self.position += 1;
            } else if byte == b'.' && !decimal_seen {
                decimal_seen = true;
                self.position += 1;
            } else {
                break;
            }
        }
        (self.position > start).then(|| {
            std::str::from_utf8(&self.input[start..self.position])
                .ok()?
                .parse()
                .ok()
        })?
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(|byte| byte.is_ascii_whitespace()) {
            self.position += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.position).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respects_operator_precedence_and_parentheses() {
        assert_eq!(evaluate("2 + 3 * 4"), Some("14".into()));
        assert_eq!(evaluate("(2 + 3) * 4"), Some("20".into()));
    }

    #[test]
    fn supports_decimals_unary_modulo_and_power() {
        assert_eq!(evaluate("-2.5 + 1"), Some("-1.5".into()));
        assert_eq!(evaluate("10 % 4"), Some("2".into()));
        assert_eq!(evaluate("2 ^ 3 ^ 2"), Some("512".into()));
    }

    #[test]
    fn rejects_invalid_or_unsafe_results() {
        assert_eq!(evaluate("hello"), None);
        assert_eq!(evaluate("2 / 0"), None);
        assert_eq!(evaluate("1 +"), None);
        assert_eq!(evaluate("(1 + 2"), None);
    }
}
