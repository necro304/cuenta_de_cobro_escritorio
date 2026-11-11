//! Contrato serde (JSON idéntico a src/types/cuentaDeCobro.ts).
use serde::{Deserialize, Serialize, Serializer, ser::SerializeMap};
use serde_json::Value;

/// Número JS que puede llegar como cualquier JSON: número → Some(f64); null/otro → None.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct JsNum(pub Option<f64>);

impl<'de> Deserialize<'de> for JsNum {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        Ok(JsNum(if v.is_number() { v.as_f64() } else { None }))
    }
}
impl Serialize for JsNum {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            None => s.serialize_none(),
            Some(x) if crate::sqlvalue::is_safe_integer(x) => s.serialize_i64(x as i64),
            Some(x) if x.is_finite() => s.serialize_f64(x),
            Some(_) => s.serialize_none(),
        }
    }
}
impl JsNum {
    /// Rowid consultable si el número es entero y cabe en i64 (equivale a enlazar el double en SQLite).
    pub fn as_rowid(&self) -> Option<i64> {
        self.0
            .filter(|x| x.is_finite() && x.fract() == 0.0 && x.abs() < 9.2e18)
            .map(|x| x as i64)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConceptoDeCobroDraft {
    pub description: String,
    pub quantity: String,
    pub price: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CuentaDeCobroDraft {
    pub number: String,
    pub date: String,
    #[serde(default)]
    pub client_id: JsNum,
    #[serde(default)]
    pub bank_account_id: JsNum,
    pub notes: String,
    pub concepts: Vec<ConceptoDeCobroDraft>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum CuentaDeCobroTarget {
    #[serde(rename = "create")]
    Create,
    #[serde(rename = "edit")]
    Edit {
        #[serde(default)]
        id: JsNum,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum SaveCuentaDeCobroCommand {
    #[serde(rename = "create")]
    Create { cuenta: CuentaDeCobroDraft },
    #[serde(rename = "edit")]
    Edit {
        #[serde(default)]
        id: JsNum,
        cuenta: CuentaDeCobroDraft,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ClienteOption {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CuentaBancariaOption {
    pub id: i64,
    pub bank: String,
    pub account_type: String,
    pub account_number: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum EstadoPago {
    #[serde(rename = "draft")]
    Draft,
    #[serde(rename = "partially_paid")]
    PartiallyPaid,
    #[serde(rename = "paid")]
    Paid,
}
impl EstadoPago {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::PartiallyPaid => "partially_paid",
            Self::Paid => "paid",
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CuentaDeCobroSummary {
    pub total: String,
    pub paid_amount: String,
    pub balance: String,
    pub payment_status: EstadoPago,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "code")]
pub enum CuentaDeCobroDiagnostic {
    #[serde(rename = "HISTORIC_NUMBER_CONFLICT")]
    HistoricNumberConflict { number: Value },
    #[serde(rename = "MISSING_CLIENT_REFERENCE", rename_all = "camelCase")]
    MissingClientReference { client_id: Value },
    #[serde(rename = "MISSING_BANK_ACCOUNT_REFERENCE", rename_all = "camelCase")]
    MissingBankAccountReference { bank_account_id: Value },
    #[serde(rename = "INVALID_PERSISTED_ACCOUNT")]
    InvalidPersistedAccount,
    #[serde(rename = "INVALID_PERSISTED_CONCEPTS")]
    InvalidPersistedConcepts,
    #[serde(rename = "INVALID_PERSISTED_PAYMENTS")]
    InvalidPersistedPayments,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CuentaDeCobroEditorModel {
    pub target: CuentaDeCobroTarget,
    pub cuenta: CuentaDeCobroDraft,
    pub clientes: Vec<ClienteOption>,
    pub cuentas_bancarias: Vec<CuentaBancariaOption>,
    pub summary: CuentaDeCobroSummary,
    pub diagnostics: Vec<CuentaDeCobroDiagnostic>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum ValidationCode {
    #[serde(rename = "REQUIRED")]
    Required,
    #[serde(rename = "INVALID_POSITIVE_INTEGER")]
    InvalidPositiveInteger,
    #[serde(rename = "INVALID_DATE")]
    InvalidDate,
    #[serde(rename = "INVALID_POSITIVE_DECIMAL")]
    InvalidPositiveDecimal,
    #[serde(rename = "AT_LEAST_ONE_CONCEPT_REQUIRED")]
    AtLeastOneConceptRequired,
    #[serde(rename = "SUBTOTAL_ROUNDS_TO_ZERO")]
    SubtotalRoundsToZero,
    #[serde(rename = "AMOUNT_OUT_OF_RANGE")]
    AmountOutOfRange,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ValidationIssue {
    pub field: String,
    pub code: ValidationCode,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "code")]
pub enum CuentaDeCobroError {
    #[serde(rename = "VALIDATION_FAILED")]
    ValidationFailed { issues: Vec<ValidationIssue> },
    #[serde(rename = "CUENTA_DE_COBRO_NOT_FOUND")]
    CuentaDeCobroNotFound { id: JsNum },
    #[serde(rename = "CLIENTE_NOT_FOUND")]
    ClienteNotFound { id: i64 },
    #[serde(rename = "CUENTA_BANCARIA_NOT_FOUND")]
    CuentaBancariaNotFound { id: i64 },
    #[serde(rename = "NUMBER_IN_USE", rename_all = "camelCase")]
    NumberInUse {
        number: i64,
        suggested_number: Option<i64>,
    },
    #[serde(rename = "TOTAL_BELOW_PAID_AMOUNT", rename_all = "camelCase")]
    TotalBelowPaidAmount { total: String, paid_amount: String },
    #[serde(rename = "STORAGE_FAILURE")]
    StorageFailure,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SaveCuentaDeCobroReceipt {
    pub id: i64,
    pub number: i64,
    pub total: String,
    pub status: EstadoPago,
}

/// { ok: true, value } | { ok: false, error }
#[derive(Debug, Clone, PartialEq)]
pub enum CuentaDeCobroResult<T> {
    Ok(T),
    Err(CuentaDeCobroError),
}

impl<T: Serialize> Serialize for CuentaDeCobroResult<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(2))?;
        match self {
            Self::Ok(v) => {
                m.serialize_entry("ok", &true)?;
                m.serialize_entry("value", v)?;
            }
            Self::Err(e) => {
                m.serialize_entry("ok", &false)?;
                m.serialize_entry("error", e)?;
            }
        }
        m.end()
    }
}
