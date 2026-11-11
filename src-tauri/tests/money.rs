//! Valores esperados capturados ejecutando src/lib/cuentaDeCobroMoney.ts y electron/cuentaDeCobro.ts (V8 / Electron 41).
use cuentacobro_lib::cuenta_de_cobro::{is_valid_date, parse_positive_safe_integer};
use cuentacobro_lib::money::*;

fn pd(s: &str, allow_zero: bool) -> Option<(String, u32, String)> {
    parse_decimal(s, allow_zero).map(|d| (d.coefficient.to_string(), d.scale, d.canonical))
}
fn some(c: &str, s: u32, k: &str) -> Option<(String, u32, String)> {
    Some((c.into(), s, k.into()))
}
fn rp(a: &str, b: &str) -> Option<Cents> {
    round_product_to_cents(
        &parse_decimal(a, true).unwrap(),
        &parse_decimal(b, true).unwrap(),
    )
}
fn d2n(s: &str) -> Option<f64> {
    decimal_to_database_number(&parse_decimal(s, true).unwrap())
}

#[test]
fn parse_decimal_casos() {
    assert_eq!(pd(" 1.50 ", false), some("15", 1, "1.5"));
    assert_eq!(pd("0001.2300", false), some("123", 2, "1.23"));
    assert_eq!(pd("0.05", false), some("5", 2, "0.05"));
    assert_eq!(pd("10.50", false), some("105", 1, "10.5"));
    assert_eq!(pd("1\u{00A0}", false), some("1", 0, "1"));
    assert_eq!(pd("\u{FEFF}1", false), some("1", 0, "1"));
    assert_eq!(pd("\u{2028}1\u{3000}", false), some("1", 0, "1"));
    for bad in [
        "1.",
        ".5",
        "000",
        "0.000",
        "1e5",
        "\u{0661}",
        "1\u{0085}",
        "-1",
        "+1",
        "1,5",
        "",
    ] {
        assert_eq!(pd(bad, false), None, "{bad:?}");
    }
    assert_eq!(pd("000", true), some("0", 0, "0"));
    assert_eq!(pd("0.000", true), some("0", 0, "0"));
}

#[test]
fn redondeo_half_up() {
    assert_eq!(rp("1", "0.005"), Some(1));
    assert_eq!(rp("1", "0.0049999"), Some(0));
    assert_eq!(rp("1", "2.675"), Some(268));
    assert_eq!(rp("3", "0.335"), Some(101));
    assert_eq!(rp("1", "0.004"), Some(0));
    assert_eq!(rp("1", "0.015"), Some(2));
    assert_eq!(rp("0.5", "0.01"), Some(1));
    assert_eq!(rp("1", "90071992547409.91"), Some(9_007_199_254_740_991));
    assert_eq!(rp("1", "90071992547409.92"), None);
    assert_eq!(rp("1", "90071992547409.915"), None);
}

#[test]
fn ida_y_vuelta_sqlite() {
    assert_eq!(d2n("0.1"), Some(0.1));
    assert_eq!(d2n("0.335"), Some(0.335));
    assert_eq!(d2n("90071992547409.91"), None);
    assert_eq!(d2n("90071992547409.9"), Some(90071992547409.9));
    assert_eq!(d2n("12345678901234567"), None);
    assert_eq!(d2n("1234567890123456.7"), None);
    assert_eq!(d2n("100000000000000000000000"), Some(1e23));
    assert_eq!(d2n("0.0000001"), Some(1e-7));
    assert_eq!(d2n(&format!("1{}", "0".repeat(400))), None);
    assert_eq!(d2n(&format!("0.{}5", "0".repeat(323))), Some(5e-324));
    assert_eq!(d2n(&format!("0.{}5", "0".repeat(330))), None);
    assert_eq!(d2n("9007199254740993"), None);
    assert_eq!(d2n("123456789.123456789"), None);
    // Empates de dígito más corto: V8 (y ryu-js) conservan el valor; Display de Rust lo rechazaría.
    assert_eq!(d2n("21027122254141.812"), Some(21027122254141.812));
    assert_eq!(d2n("664754940030335.2"), Some(664754940030335.2));
}

#[test]
fn numeros_de_bd() {
    let dec = |x: f64| database_number_to_decimal(x).map(|d| d.canonical);
    assert_eq!(
        (dec(0.0), database_number_to_cents(0.0)),
        (Some("0".into()), Some(0))
    );
    assert_eq!(
        (dec(-0.0), database_number_to_cents(-0.0)),
        (Some("0".into()), Some(0))
    );
    assert_eq!(
        (dec(0.1 + 0.2), database_number_to_cents(0.1 + 0.2)),
        (Some("0.30000000000000004".into()), Some(30))
    );
    assert_eq!(
        (dec(1e21), database_number_to_cents(1e21)),
        (Some("1000000000000000000000".into()), None)
    );
    assert_eq!(
        (dec(1e-7), database_number_to_cents(1e-7)),
        (Some("0.0000001".into()), Some(0))
    );
    assert_eq!(dec(1.23e-18), Some("0.00000000000000000123".into()));
    assert_eq!(database_number_to_cents(0.675), Some(68));
    assert_eq!(database_number_to_cents(0.665), Some(67));
    assert_eq!(database_number_to_cents(1.005), Some(101));
    assert_eq!(database_number_to_cents(2.5), Some(250));
    assert_eq!(database_number_to_cents(1e300), None);
    for bad in [-1.0, f64::NAN, f64::INFINITY] {
        assert_eq!((dec(bad), database_number_to_cents(bad)), (None, None));
    }
}

#[test]
fn centavos() {
    assert_eq!(
        (cents_to_money(0), cents_to_database_number(0)),
        ("0.00".into(), Some(0.0))
    );
    assert_eq!(
        (cents_to_money(5), cents_to_database_number(5)),
        ("0.05".into(), Some(0.05))
    );
    assert_eq!(
        (cents_to_money(100), cents_to_database_number(100)),
        ("1.00".into(), Some(1.0))
    );
    assert_eq!(
        (cents_to_money(123456), cents_to_database_number(123456)),
        ("1234.56".into(), Some(1234.56))
    );
    assert_eq!(
        (
            cents_to_money(9_007_199_254_740_991),
            cents_to_database_number(9_007_199_254_740_991)
        ),
        ("90071992547409.91".into(), None)
    );
    assert_eq!(
        (
            cents_to_money(9_007_199_254_740_990),
            cents_to_database_number(9_007_199_254_740_990)
        ),
        ("90071992547409.90".into(), Some(90071992547409.9))
    );
    assert_eq!(
        (
            cents_to_money(9_007_199_254_740_900),
            cents_to_database_number(9_007_199_254_740_900)
        ),
        ("90071992547409.00".into(), Some(90071992547409.0))
    );
}

#[test]
fn number_to_string_js() {
    for (x, s) in [
        (1e21, "1e+21"),
        (1e-7, "1e-7"),
        (123456789012345680000.0, "123456789012345680000"),
        (0.000001, "0.000001"),
        (1.5e-6, "0.0000015"),
        (-0.0, "0"),
        (9007199254740992.0, "9007199254740992"),
        (f64::MAX, "1.7976931348623157e+308"),
        (1.0, "1"),
        (-5.0, "-5"),
        (0.1 + 0.2, "0.30000000000000004"),
    ] {
        assert_eq!(js_number_to_string(x), s);
    }
}

#[test]
fn fechas_y_numeros() {
    for (d, ok) in [
        ("0050-01-01", false),
        ("0099-12-31", false),
        ("0100-01-01", true),
        ("0000-01-01", false),
        ("2024-02-29", true),
        ("2023-02-29", false),
        ("1900-02-29", false),
        ("2000-02-29", true),
        ("2026-02-30", false),
        (" 2026-01-01", false),
        ("2026-1-01", false),
        ("9999-12-31", true),
        ("2026-00-10", false),
        ("2026-13-01", false),
        ("2026-04-31", false),
        ("2026-12-31\n", false),
        ("\u{0661}\u{0662}\u{0663}\u{0664}-01-01", false),
    ] {
        assert_eq!(is_valid_date(d), ok, "{d:?}");
    }
    for (s, n) in [
        (" 12 ", Some(12)),
        ("007", Some(7)),
        ("0", None),
        ("9007199254740991", Some(9007199254740991)),
        ("9007199254740992", None),
        ("1.0", None),
        ("+1", None),
        ("1e3", None),
        ("\u{FEFF}5", Some(5)),
        ("5\u{0085}", None),
        ("00000000000000000000001", Some(1)),
    ] {
        assert_eq!(parse_positive_safe_integer(s), n, "{s:?}");
    }
}
