//! 通讯采集模块：结构化错误（用于前端稳定展示）。

use serde::{Deserialize, Serialize};

use super::union_spec_v1::AddressBase;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommRunErrorKind {
    #[serde(rename = "ConfigError")]
    ConfigError,
    #[serde(rename = "RunNotFound")]
    RunNotFound,
    #[serde(rename = "InternalError")]
    InternalError,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CommRunErrorDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missing_fields: Option<Vec<CommMissingField>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CommMissingField {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub point_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hmi_name: Option<String>,
    pub field: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommRunError {
    pub kind: CommRunErrorKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<CommRunErrorDetails>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImportUnionErrorKind {
    #[serde(rename = "UnionXlsxReadError")]
    UnionXlsxReadError,
    #[serde(rename = "UnionXlsxInvalidSheet")]
    UnionXlsxInvalidSheet,
    #[serde(rename = "UnionXlsxMissingColumns")]
    UnionXlsxMissingColumns,
    #[serde(rename = "UnionXlsxInvalidEnum")]
    UnionXlsxInvalidEnum,
    #[serde(rename = "UnionXlsxInvalidRow")]
    UnionXlsxInvalidRow,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportUnionErrorDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheet_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detected_sheets: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detected_columns: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missing_columns: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_index: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_values: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address_base_used: Option<AddressBase>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportUnionError {
    pub kind: ImportUnionErrorKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<ImportUnionErrorDetails>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PlcBridgeErrorKind {
    #[serde(rename = "CommIrReadError")]
    CommIrReadError,
    #[serde(rename = "CommIrDeserializeError")]
    CommIrDeserializeError,
    #[serde(rename = "CommIrUnsupportedSchemaVersion")]
    CommIrUnsupportedSchemaVersion,
    #[serde(rename = "CommIrUnsupportedSpecVersion")]
    CommIrUnsupportedSpecVersion,
    #[serde(rename = "CommIrValidationError")]
    CommIrValidationError,
    #[serde(rename = "PlcBridgeWriteError")]
    PlcBridgeWriteError,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlcBridgeErrorDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ir_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub point_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hmi_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_values: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlcBridgeError {
    pub kind: PlcBridgeErrorKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<PlcBridgeErrorDetails>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BridgeCheckErrorKind {
    #[serde(rename = "PlcBridgeReadError")]
    PlcBridgeReadError,
    #[serde(rename = "PlcBridgeDeserializeError")]
    PlcBridgeDeserializeError,
    #[serde(rename = "PlcBridgeUnsupportedSchemaVersion")]
    PlcBridgeUnsupportedSchemaVersion,
    #[serde(rename = "PlcBridgeUnsupportedSpecVersion")]
    PlcBridgeUnsupportedSpecVersion,
    #[serde(rename = "PlcBridgeValidationError")]
    PlcBridgeValidationError,
    #[serde(rename = "BridgeSummaryWriteError")]
    BridgeSummaryWriteError,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct BridgeCheckErrorDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BridgeCheckError {
    pub kind: BridgeCheckErrorKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<BridgeCheckErrorDetails>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImportResultStubErrorKind {
    #[serde(rename = "PlcBridgeReadError")]
    PlcBridgeReadError,
    #[serde(rename = "PlcBridgeDeserializeError")]
    PlcBridgeDeserializeError,
    #[serde(rename = "PlcBridgeUnsupportedSchemaVersion")]
    PlcBridgeUnsupportedSchemaVersion,
    #[serde(rename = "PlcBridgeUnsupportedSpecVersion")]
    PlcBridgeUnsupportedSpecVersion,
    #[serde(rename = "ImportResultStubValidationError")]
    ImportResultStubValidationError,
    #[serde(rename = "ImportResultStubWriteError")]
    ImportResultStubWriteError,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportResultStubErrorDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportResultStubError {
    pub kind: ImportResultStubErrorKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<ImportResultStubErrorDetails>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MergeImportSourcesErrorKind {
    #[serde(rename = "UnionXlsxReadError")]
    UnionXlsxReadError,
    #[serde(rename = "UnionXlsxParseError")]
    UnionXlsxParseError,
    #[serde(rename = "UnionXlsxValidationError")]
    UnionXlsxValidationError,
    #[serde(rename = "ImportResultStubReadError")]
    ImportResultStubReadError,
    #[serde(rename = "ImportResultStubDeserializeError")]
    ImportResultStubDeserializeError,
    #[serde(rename = "ImportResultStubUnsupportedSchemaVersion")]
    ImportResultStubUnsupportedSchemaVersion,
    #[serde(rename = "ImportResultStubUnsupportedSpecVersion")]
    ImportResultStubUnsupportedSpecVersion,
    #[serde(rename = "ImportResultStubValidationError")]
    ImportResultStubValidationError,
    #[serde(rename = "MergeWriteError")]
    MergeWriteError,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct MergeImportSourcesErrorDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub union_xlsx_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_result_stub_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MergeImportSourcesError {
    pub kind: MergeImportSourcesErrorKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<MergeImportSourcesErrorDetails>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum UnifiedPlcImportStubErrorKind {
    #[serde(rename = "UnifiedImportReadError")]
    UnifiedImportReadError,
    #[serde(rename = "UnifiedImportDeserializeError")]
    UnifiedImportDeserializeError,
    #[serde(rename = "UnifiedImportUnsupportedSchemaVersion")]
    UnifiedImportUnsupportedSchemaVersion,
    #[serde(rename = "UnifiedImportUnsupportedSpecVersion")]
    UnifiedImportUnsupportedSpecVersion,
    #[serde(rename = "UnifiedImportValidationError")]
    UnifiedImportValidationError,
    #[serde(rename = "PlcImportStubWriteError")]
    PlcImportStubWriteError,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedPlcImportStubErrorDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unified_import_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_values: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedPlcImportStubError {
    pub kind: UnifiedPlcImportStubErrorKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<UnifiedPlcImportStubErrorDetails>,
}
