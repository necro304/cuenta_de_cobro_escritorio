//! Port de src/lib/cuentaDeCobroMoney.ts (paridad exacta con la versión JS).
use num_bigint::BigUint;
use num_traits::{ToPrimitive, Zero};

pub type Cents = u64;
/// BigInt(Number.MAX_SAFE_INTEGER)
pub const MAX_SAFE_CENTS: Cents = 9_007_199_254_740_991;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDecimal {
    pub coefficient: BigUint,
    pub scale: u32,
    pub canonical: String,
}

/// Conjunto exacto de String.prototype.trim (WhiteSpace + LineTerminator de ECMAScript).
pub fn is_js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'
            | '\u{000A}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

pub fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_whitespace)
}

/// Number.prototype.toString() de ECMAScript (ryu-js es conforme a la especificación).
pub fn js_number_to_string(x: f64) -> String {
    let mut buffer = ryu_js::Buffer::new();
    buffer.format(x).to_string()
}

pub fn cents_to_money(cents: Cents) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}

/// /^(\d+)(?:\.(\d+))?$/ sobre el texto recortado; \d es ASCII [0-9].
pub fn parse_decimal(input: &str, allow_zero: bool) -> Option<ParsedDecimal> {
    let value = js_trim(input);
    let (int_raw, frac_raw) = match value.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (value, None),
    };
    let all_digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !all_digits(int_raw) {
        return None;
    }
    if let Some(f) = frac_raw
        && !all_digits(f)
    {
        return None;
    }

    // replace(/^0+(?=\d)/, '') : quita ceros a la izquierda dejando al menos un dígito
    let trimmed_int = int_raw.trim_start_matches('0');
    let integer_part = if trimmed_int.is_empty() {
        "0"
    } else {
        trimmed_int
    };
    // replace(/0+$/, '') : quita ceros finales de la parte fraccionaria
    let fraction_part = frac_raw.unwrap_or("").trim_end_matches('0');

    let coefficient =
        BigUint::parse_bytes(format!("{integer_part}{fraction_part}").as_bytes(), 10)?;
    if coefficient.is_zero() && !allow_zero {
        return None;
    }

    Some(ParsedDecimal {
        coefficient,
        scale: fraction_part.len() as u32,
        canonical: if fraction_part.is_empty() {
            integer_part.to_string()
        } else {
            format!("{integer_part}.{fraction_part}")
        },
    })
}

/// Producto exacto redondeado half-up a centavos; None si supera MAX_SAFE_CENTS.
pub fn round_product_to_cents(left: &ParsedDecimal, right: &ParsedDecimal) -> Option<Cents> {
    let product = &left.coefficient * &right.coefficient;
    let scale = left.scale + right.scale;
    let ten = BigUint::from(10u32);
    let cents = if scale <= 2 {
        product * ten.pow(2 - scale)
    } else {
        let divisor = ten.pow(scale - 2);
        let remainder = &product % &divisor;
        let mut cents = &product / &divisor;
        if remainder * 2u32 >= divisor {
            cents += 1u32;
        }
        cents
    };
    if cents <= BigUint::from(MAX_SAFE_CENTS) {
        cents.to_u64()
    } else {
        None
    }
}

/// expandExponential: notación decimal completa de Number#toString; None si no es finito o es < 0.
pub fn expand_exponential(value: f64) -> Option<String> {
    if !value.is_finite() || value < 0.0 {
        return None;
    } // -0.0 < 0.0 es false: -0 pasa (como en JS)
    let serialized = js_number_to_string(value).to_lowercase();
    let Some((mantissa, exponent_text)) = serialized.split_once('e') else {
        return Some(serialized);
    };
    let exponent: i64 = exponent_text.parse().ok()?; // "+21" / "-7"
    let integer_length = mantissa.find('.').unwrap_or(mantissa.len()) as i64;
    let digits = mantissa.replacen('.', "", 1);
    let digits_len = digits.len() as i64;
    let expanded_index = integer_length + exponent;
    if expanded_index <= 0 {
        return Some(format!(
            "0.{}{}",
            "0".repeat((-expanded_index) as usize),
            digits
        ));
    }
    if expanded_index >= digits_len {
        return Some(format!(
            "{}{}",
            digits,
            "0".repeat((expanded_index - digits_len) as usize)
        ));
    }
    let i = expanded_index as usize;
    Some(format!("{}.{}", &digits[..i], &digits[i..]))
}

pub fn database_number_to_decimal(value: f64) -> Option<ParsedDecimal> {
    parse_decimal(&expand_exponential(value)?, true)
}

/// Number(canonical) y comprobación de ida y vuelta; None si SQLite (REAL) alteraría el valor.
pub fn decimal_to_database_number(decimal: &ParsedDecimal) -> Option<f64> {
    let value: f64 = decimal.canonical.parse().ok()?; // correctamente redondeado, como Number()
    let restored = database_number_to_decimal(value)?;
    (restored.canonical == decimal.canonical).then_some(value)
}

pub fn database_number_to_cents(value: f64) -> Option<Cents> {
    let decimal = database_number_to_decimal(value)?;
    let one = ParsedDecimal {
        coefficient: BigUint::from(1u32),
        scale: 0,
        canonical: "1".into(),
    };
    round_product_to_cents(&decimal, &one)
}

pub fn cents_to_database_number(cents: Cents) -> Option<f64> {
    decimal_to_database_number(&parse_decimal(&cents_to_money(cents), true)?)
}
