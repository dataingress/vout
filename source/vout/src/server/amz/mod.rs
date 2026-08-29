mod delete_parameter;
mod delete_parameters;
mod describe_parameters;
mod get_parameter;
mod get_parameter_history;
mod get_parameters;
mod get_parameters_by_path;
mod label_parameter_version;
mod put_parameter;
mod unlabel_parameter_version;

pub const AMZ_SSM_DELETE_PARAMETER: &str = "AmazonSSM.DeleteParameter";
pub const AMZ_SSM_DELETE_PARAMETERS: &str = "AmazonSSM.DeleteParameters";
pub const AMZ_SSM_DESCRIBE_PARAMETERS: &str = "AmazonSSM.DescribeParameters";
pub const AMZ_SSM_GET_PARAMETER: &str = "AmazonSSM.GetParameter";
pub const AMZ_SSM_GET_PARAMETER_HISTORY: &str = "AmazonSSM.GetParameterHistory";
pub const AMZ_SSM_GET_PARAMETERS: &str = "AmazonSSM.GetParameters";
pub const AMZ_SSM_GET_PARAMETERS_BY_PATH: &str = "AmazonSSM.GetParametersByPath";
pub const AMZ_SSM_LABEL_PARAMETER_VERSION: &str = "AmazonSSM.LabelParameterVersion";
pub const AMZ_SSM_PUT_PARAMETER: &str = "AmazonSSM.PutParameter";
pub const AMZ_SSM_UNLABEL_PARAMETER_VERSION: &str = "AmazonSSM.UnlabelParameterVersion";

pub use delete_parameter::handler as delete_parameter;
pub use delete_parameters::handler as delete_parameters;
pub use describe_parameters::handler as describe_parameters;
pub use get_parameter::handler as get_parameter;
pub use get_parameter_history::handler as get_parameter_history;
pub use get_parameters::handler as get_parameters;
pub use get_parameters_by_path::handler as get_parameters_by_path;
pub use label_parameter_version::handler as label_parameter_version;
pub use put_parameter::handler as put_parameter;
pub use unlabel_parameter_version::handler as unlabel_parameter_version;

use base64::Engine;
use http_body_util::Full;
use hyper::Response;
use hyper::body::Bytes;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use sha2::{Digest, Sha256};

const DEFAULT_DATA_TYPE: &str = "text";
const DEFAULT_TIER: &str = "Standard";
const MAX_LABELS_PER_VERSION: usize = 10;
const SIGV4_ALGORITHM: &str = "AWS4-HMAC-SHA256";
const MAX_PARAMETER_NAME_LEN: usize = 2048;
const MAX_PARAMETER_VALUE_LEN: usize = 8192;
const MAX_DESCRIPTION_LEN: usize = 1024;
const MAX_ALLOWED_PATTERN_LEN: usize = 1024;
const MAX_TAGS: usize = 50;
const MAX_TAG_KEY_LEN: usize = 128;
const MAX_TAG_VALUE_LEN: usize = 256;
const MAX_GET_PARAMETERS_NAMES: usize = 10;
const MAX_DELETE_PARAMETERS_NAMES: usize = 10;
const MAX_FILTERS: usize = 10;
const MAX_FILTER_VALUES: usize = 50;

pub type AmzRequest = hyper::Request<Full<Bytes>>;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Tag {
    pub key: String,
    pub value: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum ParameterType {
    SecureString,
    String,
    StringList,
}

impl ParameterType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ParameterType::SecureString => "SecureString",
            ParameterType::String => "String",
            ParameterType::StringList => "StringList",
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ParameterStringFilter {
    pub key: String,
    pub option: Option<String>,
    pub values: Option<Vec<String>>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Parameter {
    pub name: String,
    pub r#type: String,
    pub value: String,
    pub version: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
    pub last_modified_date: f64,
    pub arn: String,
    pub data_type: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ParameterMetadata {
    pub name: String,
    pub r#type: String,
    pub version: i64,
    pub last_modified_date: f64,
    pub arn: String,
    pub data_type: String,
    pub tier: &'static str,
    pub policies: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_pattern: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ParameterHistory {
    pub name: String,
    pub r#type: String,
    pub value: String,
    pub version: i64,
    pub last_modified_date: f64,
    pub data_type: String,
    pub tier: &'static str,
    pub policies: Vec<serde_json::Value>,
    pub labels: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_pattern: Option<String>,
}

pub enum ParameterSelector {
    Latest(String),
    Version { name: String, version: i64 },
    Label { name: String, label: String },
}

impl ParameterSelector {
    pub fn name(&self) -> &str {
        match self {
            ParameterSelector::Latest(name)
            | ParameterSelector::Version { name, .. }
            | ParameterSelector::Label { name, .. } => name,
        }
    }

    pub fn selector(&self) -> Option<String> {
        match self {
            ParameterSelector::Latest(_) => None,
            ParameterSelector::Version { version, .. } => Some(format!(":{version}")),
            ParameterSelector::Label { label, .. } => Some(format!(":{label}")),
        }
    }
}

pub fn parse_selector(value: &str) -> ParameterSelector {
    if let Some((name, selector)) = value.rsplit_once(':')
        && !name.starts_with("arn:")
        && !selector.is_empty()
    {
        if let Ok(version) = selector.parse::<i64>() {
            return ParameterSelector::Version {
                name: name.to_owned(),
                version,
            };
        }

        return ParameterSelector::Label {
            name: name.to_owned(),
            label: selector.to_owned(),
        };
    }

    ParameterSelector::Latest(value.to_owned())
}

pub fn json_response<T: serde::Serialize>(
    value: &T,
) -> anyhow::Result<crate::server::res::GwPassed<'static>> {
    Ok(crate::server::res::GwPassed::Success(Response::new(
        Full::new(Bytes::from(serde_json::to_string(value)?)),
    )))
}

pub fn empty_json_response() -> crate::server::res::GwPassed<'static> {
    crate::server::res::GwPassed::Success(Response::new(Full::new(Bytes::from("{}"))))
}

pub fn arn(name: &str) -> String {
    format!("arn:aws:ssm:local:000000000000:parameter/{name}")
}

pub fn timestamp(value: chrono::DateTime<chrono::Utc>) -> f64 {
    value.timestamp_millis() as f64 / 1000.0
}

pub fn page_bounds(
    next_token: Option<&str>,
    max_results: Option<u64>,
    default_max: usize,
    max_max: usize,
) -> Option<(usize, usize)> {
    let offset = match next_token {
        Some(token) => token.parse::<usize>().ok()?,
        None => 0,
    };
    let limit = max_results.unwrap_or(default_max as u64) as usize;

    Some((offset, limit.clamp(1, max_max)))
}

pub fn take_page<T>(
    mut items: Vec<T>,
    offset: usize,
    limit: usize,
) -> Option<(Vec<T>, Option<String>)> {
    if offset > items.len() {
        return None;
    }

    let next_offset = offset + limit;
    let next_token = (next_offset < items.len()).then(|| next_offset.to_string());
    let end = next_offset.min(items.len());

    Some((items.drain(offset..end).collect(), next_token))
}

pub fn valid_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= 100
        && !label.chars().next().is_some_and(|v| v.is_ascii_digit())
        && label
            .chars()
            .all(|v| v.is_ascii_alphanumeric() || matches!(v, '_' | '-' | '.'))
}

pub fn validate_parameter_name(name: &str) -> Option<crate::server::res::GwPassed<'static>> {
    if name.is_empty() || name.len() > MAX_PARAMETER_NAME_LEN || name.contains(char::is_whitespace)
    {
        return Some(crate::server::err::builder::invalid_parameter(
            "Parameter name is invalid.",
        ));
    }

    None
}

pub fn validate_parameter_names(
    names: &[String],
    max: usize,
) -> Option<crate::server::res::GwPassed<'static>> {
    if names.is_empty() || names.len() > max {
        return Some(crate::server::err::builder::invalid_parameter(
            "Invalid number of parameter names.",
        ));
    }

    names.iter().find_map(|name| validate_parameter_name(name))
}

pub fn validate_put_request(
    name: &str,
    value: &str,
    description: Option<&str>,
    allowed_pattern: Option<&str>,
    tags: Option<&[Tag]>,
) -> Option<crate::server::res::GwPassed<'static>> {
    if let Some(err) = validate_parameter_name(name) {
        return Some(err);
    }

    if value.len() > MAX_PARAMETER_VALUE_LEN {
        return Some(crate::server::err::builder::invalid_parameter(
            "Parameter value is too long.",
        ));
    }

    if description.is_some_and(|value| value.len() > MAX_DESCRIPTION_LEN) {
        return Some(crate::server::err::builder::invalid_parameter(
            "Parameter description is too long.",
        ));
    }

    if allowed_pattern.is_some_and(|value| value.len() > MAX_ALLOWED_PATTERN_LEN) {
        return Some(crate::server::err::builder::invalid_parameter(
            "AllowedPattern is too long.",
        ));
    }

    validate_tags(tags.unwrap_or(&[]))
}

pub fn validate_tags(tags: &[Tag]) -> Option<crate::server::res::GwPassed<'static>> {
    if tags.len() > MAX_TAGS {
        return Some(crate::server::err::builder::invalid_parameter(
            "Too many tags.",
        ));
    }

    for tag in tags {
        if tag.key.is_empty()
            || tag.key.len() > MAX_TAG_KEY_LEN
            || tag.value.len() > MAX_TAG_VALUE_LEN
        {
            return Some(crate::server::err::builder::invalid_parameter(
                "Tag key or value is invalid.",
            ));
        }
    }

    None
}

pub fn validate_labels(labels: &[String]) -> Option<crate::server::res::GwPassed<'static>> {
    if labels.is_empty() || labels.len() > MAX_LABELS_PER_VERSION {
        return Some(crate::server::err::builder::invalid_parameter(
            "Invalid number of labels.",
        ));
    }

    None
}

pub fn validate_filters(
    filters: Option<&[ParameterStringFilter]>,
) -> Option<crate::server::res::GwPassed<'static>> {
    let filters = filters?;

    if filters.len() > MAX_FILTERS {
        return Some(crate::server::err::builder::invalid_filter_value());
    }

    for filter in filters {
        if filter.key.is_empty() {
            return Some(crate::server::err::builder::invalid_filter_key());
        }

        if filter
            .values
            .as_ref()
            .is_some_and(|values| values.len() > MAX_FILTER_VALUES)
        {
            return Some(crate::server::err::builder::invalid_filter_value());
        }
    }

    None
}

pub fn max_get_parameters_names() -> usize {
    MAX_GET_PARAMETERS_NAMES
}

pub fn max_delete_parameters_names() -> usize {
    MAX_DELETE_PARAMETERS_NAMES
}

pub fn path_matches(name: &str, path: &str, recursive: bool) -> bool {
    let prefix = if path.ends_with('/') {
        path.to_owned()
    } else {
        format!("{path}/")
    };

    if !name.starts_with(&prefix) {
        return false;
    }

    recursive || !name[prefix.len()..].contains('/')
}

fn value_aad(name: &str, version: i64, parameter_type: &str) -> String {
    format!("{name}\0{version}\0{parameter_type}")
}

pub fn encode_value(
    value: &str,
    parameter_name: &str,
    version: i64,
    parameter_type: &ParameterType,
) -> anyhow::Result<Vec<u8>> {
    match parameter_type {
        ParameterType::SecureString => crate::crypto::param::encrypt(
            value.as_bytes(),
            value_aad(parameter_name, version, parameter_type.as_str()).as_bytes(),
        ),
        ParameterType::String | ParameterType::StringList => Ok(value.as_bytes().to_vec()),
    }
}

fn decode_stored_value(
    value: &[u8],
    parameter_name: &str,
    version: i64,
    parameter_type: &str,
    with_decryption: bool,
) -> anyhow::Result<String> {
    if parameter_type == ParameterType::SecureString.as_str() && with_decryption {
        return Ok(String::from_utf8(crate::crypto::param::decrypt(
            value,
            value_aad(parameter_name, version, parameter_type).as_bytes(),
        )?)?);
    }

    if parameter_type == ParameterType::SecureString.as_str() {
        return Ok(base64::prelude::BASE64_STANDARD.encode(value));
    }

    Ok(String::from_utf8(value.to_vec())?)
}

pub async fn labels_for_version<C>(
    conn: &C,
    parameter_name: &str,
    version: i64,
) -> anyhow::Result<Vec<String>>
where
    C: ConnectionTrait,
{
    let mut labels = migration::models::tb_param_label::Entity::find()
        .filter(migration::models::tb_param_label::Column::ParamKey.eq(parameter_name))
        .filter(migration::models::tb_param_label::Column::Version.eq(version))
        .all(conn)
        .await?
        .into_iter()
        .map(|label| label.label)
        .collect::<Vec<_>>();

    labels.sort();
    Ok(labels)
}

pub async fn resolve_version<C>(
    conn: &C,
    selector: &ParameterSelector,
) -> anyhow::Result<Option<migration::models::tb_param_version::Model>>
where
    C: ConnectionTrait,
{
    match selector {
        ParameterSelector::Latest(name) => {
            if let Some(param) = migration::models::tb_param::Entity::find_by_id(name.to_owned())
                .one(conn)
                .await?
            {
                return Ok(migration::models::tb_param_version::Entity::find_by_id((
                    name.to_owned(),
                    param.version,
                ))
                .one(conn)
                .await?);
            }

            Ok(None)
        }
        ParameterSelector::Version { name, version } => Ok(
            migration::models::tb_param_version::Entity::find_by_id((name.to_owned(), *version))
                .one(conn)
                .await?,
        ),
        ParameterSelector::Label { name, label } => {
            if let Some(label) = migration::models::tb_param_label::Entity::find_by_id((
                name.to_owned(),
                label.to_owned(),
            ))
            .one(conn)
            .await?
            {
                return Ok(migration::models::tb_param_version::Entity::find_by_id((
                    name.to_owned(),
                    label.version,
                ))
                .one(conn)
                .await?);
            }

            Ok(None)
        }
    }
}

pub fn parameter_from_version(
    version: migration::models::tb_param_version::Model,
    selector: Option<String>,
    with_decryption: bool,
) -> anyhow::Result<Parameter> {
    Ok(Parameter {
        name: version.param_key.clone(),
        r#type: version.r#type.clone(),
        value: decode_stored_value(
            &version.value,
            &version.param_key,
            version.version,
            &version.r#type,
            with_decryption,
        )?,
        version: version.version,
        selector,
        last_modified_date: timestamp(version.last_modified_date),
        arn: arn(&version.param_key),
        data_type: version
            .data_type
            .unwrap_or_else(|| DEFAULT_DATA_TYPE.to_owned()),
    })
}

pub fn metadata_from_param(param: migration::models::tb_param::Model) -> ParameterMetadata {
    ParameterMetadata {
        name: param.key.clone(),
        r#type: param.r#type,
        version: param.version,
        last_modified_date: timestamp(param.last_modified_date),
        arn: arn(&param.key),
        data_type: param
            .data_type
            .unwrap_or_else(|| DEFAULT_DATA_TYPE.to_owned()),
        tier: DEFAULT_TIER,
        policies: Vec::new(),
        description: param.description,
        allowed_pattern: param.allowed_pattern,
    }
}

pub async fn history_from_version<C>(
    conn: &C,
    version: migration::models::tb_param_version::Model,
    with_decryption: bool,
) -> anyhow::Result<ParameterHistory>
where
    C: ConnectionTrait,
{
    let labels = labels_for_version(conn, &version.param_key, version.version).await?;

    Ok(ParameterHistory {
        name: version.param_key.clone(),
        r#type: version.r#type.clone(),
        value: decode_stored_value(
            &version.value,
            &version.param_key,
            version.version,
            &version.r#type,
            with_decryption,
        )?,
        version: version.version,
        last_modified_date: timestamp(version.last_modified_date),
        data_type: version
            .data_type
            .unwrap_or_else(|| DEFAULT_DATA_TYPE.to_owned()),
        tier: DEFAULT_TIER,
        policies: Vec::new(),
        labels,
        description: version.description,
        allowed_pattern: version.allowed_pattern,
    })
}

pub async fn parameter_has_label<C>(
    conn: &C,
    parameter_name: &str,
    label: &str,
) -> anyhow::Result<bool>
where
    C: ConnectionTrait,
{
    Ok(migration::models::tb_param_label::Entity::find_by_id((
        parameter_name.to_owned(),
        label.to_owned(),
    ))
    .one(conn)
    .await?
    .is_some())
}

pub async fn parameter_version_has_label<C>(
    conn: &C,
    parameter_name: &str,
    version: i64,
    label: &str,
) -> anyhow::Result<bool>
where
    C: ConnectionTrait,
{
    Ok(migration::models::tb_param_label::Entity::find()
        .filter(migration::models::tb_param_label::Column::ParamKey.eq(parameter_name))
        .filter(migration::models::tb_param_label::Column::Version.eq(version))
        .filter(migration::models::tb_param_label::Column::Label.eq(label))
        .one(conn)
        .await?
        .is_some())
}

pub async fn parameter_has_tag<C>(
    conn: &C,
    parameter_name: &str,
    tag_key: &str,
    values: &[String],
) -> anyhow::Result<bool>
where
    C: ConnectionTrait,
{
    let links = migration::models::tb_param_tag::Entity::find()
        .filter(migration::models::tb_param_tag::Column::ParamKey.eq(parameter_name))
        .all(conn)
        .await?;

    for link in links {
        if let Some(tag) = migration::models::tb_tag::Entity::find_by_id(link.tag_id)
            .one(conn)
            .await?
            && tag.key == tag_key
            && (values.is_empty() || values.iter().any(|value| value == &tag.value))
        {
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn label_limit() -> usize {
    MAX_LABELS_PER_VERSION
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 0x0f) as usize] as char);
    }

    result
}

fn hex_decode(value: &str) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return None;
    }

    let mut result = Vec::with_capacity(value.len() / 2);
    let mut chars = value.bytes();

    while let (Some(high), Some(low)) = (chars.next(), chars.next()) {
        let high = (high as char).to_digit(16)?;
        let low = (low as char).to_digit(16)?;
        result.push(((high << 4) | low) as u8);
    }

    Some(result)
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK_SIZE: usize = 64;
    let mut normalized_key = [0u8; BLOCK_SIZE];

    if key.len() > BLOCK_SIZE {
        normalized_key[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        normalized_key[..key.len()].copy_from_slice(key);
    }

    let mut outer_key = [0x5c; BLOCK_SIZE];
    let mut inner_key = [0x36; BLOCK_SIZE];

    for i in 0..BLOCK_SIZE {
        outer_key[i] ^= normalized_key[i];
        inner_key[i] ^= normalized_key[i];
    }

    let mut inner = Sha256::new();
    inner.update(inner_key);
    inner.update(message);
    let inner = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(outer_key);
    outer.update(inner);
    outer.finalize().into()
}

fn sigv4_key(secret_key: &str, date: &str, region: &str, service: &str) -> [u8; 32] {
    let k_date = hmac_sha256(format!("AWS4{secret_key}").as_bytes(), date.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());

    hmac_sha256(&k_service, b"aws4_request")
}

fn parse_authorization(value: &str) -> Option<(&str, Vec<&str>, &str)> {
    let value = value.strip_prefix(SIGV4_ALGORITHM)?.trim_start();
    let mut credential = None;
    let mut signed_headers = None;
    let mut signature = None;

    for part in value.split(',') {
        let (key, value) = part.trim().split_once('=')?;

        match key {
            "Credential" => credential = Some(value),
            "SignedHeaders" => signed_headers = Some(value.split(';').collect::<Vec<_>>()),
            "Signature" => signature = Some(value),
            _ => {}
        }
    }

    Some((credential?, signed_headers?, signature?))
}

fn canonical_header_value(value: &hyper::header::HeaderValue) -> Option<String> {
    Some(
        value
            .to_str()
            .ok()?
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn canonical_query(query: Option<&str>) -> String {
    let Some(query) = query else {
        return String::new();
    };
    let mut pairs = query.split('&').collect::<Vec<_>>();
    pairs.sort();
    pairs.join("&")
}

fn canonical_request(
    req: &AmzRequest,
    signed_headers: &[&str],
    payload_hash: &str,
) -> Option<String> {
    let mut canonical_headers = String::new();

    for header in signed_headers {
        let value = req.headers().get(*header)?;
        canonical_headers.push_str(header);
        canonical_headers.push(':');
        canonical_headers.push_str(&canonical_header_value(value)?);
        canonical_headers.push('\n');
    }

    Some(format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        req.method().as_str(),
        req.uri().path(),
        canonical_query(req.uri().query()),
        canonical_headers,
        signed_headers.join(";"),
        payload_hash
    ))
}

pub async fn collect_limited_body(
    mut body: hyper::body::Incoming,
    limit: u64,
) -> Result<Bytes, crate::server::res::GwPassed<'static>> {
    use http_body_util::BodyExt;

    let mut result = Vec::new();

    while let Some(frame) = body.frame().await {
        let frame = frame.map_err(|_| crate::server::err::builder::invalid_request_body())?;

        if let Some(data) = frame.data_ref() {
            let next_len = result.len() as u64 + data.len() as u64;

            if next_len > limit {
                return Err(crate::server::err::builder::invalid_parameter(
                    "Request body is too large.",
                ));
            }

            result.extend_from_slice(data);
        }
    }

    Ok(Bytes::from(result))
}

pub async fn authenticate(
    req: &AmzRequest,
    body: &[u8],
) -> anyhow::Result<Option<crate::server::res::GwPassed<'static>>> {
    let Some(authorization) = req
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
    else {
        return Ok(Some(crate::server::err::builder::invalid_parameter(
            "Missing Authorization header.",
        )));
    };
    let Some((credential, signed_headers, signature)) = parse_authorization(authorization) else {
        return Ok(Some(crate::server::err::builder::invalid_parameter(
            "Invalid Authorization header.",
        )));
    };
    let credential_parts = credential.split('/').collect::<Vec<_>>();

    if credential_parts.len() != 5 || credential_parts[4] != "aws4_request" {
        return Ok(Some(crate::server::err::builder::invalid_parameter(
            "Invalid credential scope.",
        )));
    }

    let access_key = credential_parts[0];
    let date = credential_parts[1];
    let region = credential_parts[2];
    let service = credential_parts[3];

    if service != "ssm" {
        return Ok(Some(crate::server::err::builder::invalid_parameter(
            "Invalid credential service.",
        )));
    }

    let payload_hash = hex_encode(&Sha256::digest(body));

    if let Some(header_hash) = req
        .headers()
        .get("x-amz-content-sha256")
        .and_then(|value| value.to_str().ok())
        && header_hash != payload_hash
    {
        return Ok(Some(crate::server::err::builder::invalid_parameter(
            "Invalid payload hash.",
        )));
    }

    let Some(amz_date) = req
        .headers()
        .get("x-amz-date")
        .and_then(|value| value.to_str().ok())
    else {
        return Ok(Some(crate::server::err::builder::invalid_parameter(
            "Missing X-Amz-Date header.",
        )));
    };

    if !amz_date.starts_with(date) {
        return Ok(Some(crate::server::err::builder::invalid_parameter(
            "Invalid credential date.",
        )));
    }

    let request_time = match chrono::NaiveDateTime::parse_from_str(amz_date, "%Y%m%dT%H%M%SZ") {
        Ok(value) => value.and_utc(),
        Err(_) => {
            return Ok(Some(crate::server::err::builder::invalid_parameter(
                "Invalid X-Amz-Date header.",
            )));
        }
    };
    let skew = (chrono::Utc::now() - request_time).num_seconds().abs();

    if skew > 900 {
        return Ok(Some(crate::server::err::builder::invalid_parameter(
            "Request timestamp is outside the allowed time window.",
        )));
    }

    let Some(canonical_request) = canonical_request(req, &signed_headers, &payload_hash) else {
        return Ok(Some(crate::server::err::builder::invalid_parameter(
            "Signed header is missing.",
        )));
    };
    let canonical_request_hash = hex_encode(&Sha256::digest(canonical_request.as_bytes()));
    let string_to_sign = format!(
        "{SIGV4_ALGORITHM}\n{amz_date}\n{date}/{region}/{service}/aws4_request\n{canonical_request_hash}"
    );
    let conn = match crate::db::open().await {
        Ok(conn) => conn,
        Err(err) => return Err(err),
    };
    let Some(account) = migration::models::tb_account::Entity::find_by_id(access_key.to_owned())
        .one(&conn)
        .await?
    else {
        return Ok(Some(crate::server::err::builder::invalid_parameter(
            "The security token included in the request is invalid.",
        )));
    };

    if account
        .expires_at
        .is_some_and(|expires_at| expires_at <= chrono::Utc::now())
    {
        return Ok(Some(crate::server::err::builder::invalid_parameter(
            "The security token included in the request is expired.",
        )));
    }

    let encrypted_secret_key = match base64::prelude::BASE64_STANDARD.decode(&account.secret_key) {
        Ok(secret_key) => secret_key,
        Err(_) => {
            return Ok(Some(crate::server::err::builder::invalid_parameter(
                "The security token included in the request is invalid.",
            )));
        }
    };
    let secret_key = match crate::crypto::param::decrypt(
        &encrypted_secret_key,
        format!(
            "{}\0{}",
            crate::app::account::ACCOUNT_SECRET_AAD_PREFIX,
            account.access_key
        )
        .as_bytes(),
    ) {
        Ok(secret_key) => secret_key,
        Err(_) => {
            return Ok(Some(crate::server::err::builder::invalid_parameter(
                "The security token included in the request is invalid.",
            )));
        }
    };
    let secret_key = match String::from_utf8(secret_key) {
        Ok(secret_key) => secret_key,
        Err(_) => {
            return Ok(Some(crate::server::err::builder::invalid_parameter(
                "The security token included in the request is invalid.",
            )));
        }
    };
    let signing_key = sigv4_key(&secret_key, date, region, service);
    let expected = hmac_sha256(&signing_key, string_to_sign.as_bytes());
    let Some(provided) = hex_decode(signature) else {
        return Ok(Some(crate::server::err::builder::invalid_parameter(
            "Invalid request signature.",
        )));
    };

    if provided.len() != expected.len()
        || !provided
            .iter()
            .zip(expected)
            .all(|(left, right)| *left == right)
    {
        return Ok(Some(crate::server::err::builder::invalid_parameter(
            "The request signature we calculated does not match the signature you provided.",
        )));
    }

    Ok(None)
}

pub async fn delete_parameter_named<C>(conn: &C, parameter_name: &str) -> anyhow::Result<bool>
where
    C: ConnectionTrait,
{
    let exists = migration::models::tb_param::Entity::find_by_id(parameter_name.to_owned())
        .one(conn)
        .await?
        .is_some();

    if !exists {
        return Ok(false);
    }

    let old_links = migration::models::tb_param_tag::Entity::find()
        .filter(migration::models::tb_param_tag::Column::ParamKey.eq(parameter_name))
        .all(conn)
        .await?;
    let old_tag_ids = old_links.iter().map(|link| link.tag_id).collect::<Vec<_>>();

    migration::models::tb_param_label::Entity::delete_many()
        .filter(migration::models::tb_param_label::Column::ParamKey.eq(parameter_name))
        .exec(conn)
        .await?;
    migration::models::tb_param_version::Entity::delete_many()
        .filter(migration::models::tb_param_version::Column::ParamKey.eq(parameter_name))
        .exec(conn)
        .await?;
    migration::models::tb_param_tag::Entity::delete_many()
        .filter(migration::models::tb_param_tag::Column::ParamKey.eq(parameter_name))
        .exec(conn)
        .await?;
    migration::models::tb_param::Entity::delete_by_id(parameter_name.to_owned())
        .exec(conn)
        .await?;

    for tag_id in old_tag_ids {
        let still_used = migration::models::tb_param_tag::Entity::find()
            .filter(migration::models::tb_param_tag::Column::TagId.eq(tag_id))
            .one(conn)
            .await?
            .is_some();

        if !still_used {
            migration::models::tb_tag::Entity::delete_by_id(tag_id)
                .exec(conn)
                .await?;
        }
    }

    Ok(true)
}

#[macro_use]
pub mod internal {
    #[macro_export]
    macro_rules! deserialize_request_body_inner {
        ($req:ident, $object:ty) => {{
            let body = match http_body_util::BodyExt::collect($req.into_body()).await {
                Ok(body) => body.to_bytes(),
                Err(_) => {
                    return Ok($crate::server::err::builder::invalid_request_body());
                }
            };

            match serde_json::from_slice::<$object>(&body) {
                Ok(request) => request,
                Err(_) => {
                    return Ok($crate::server::err::builder::invalid_request_body());
                }
            }
        }};
    }

    #[macro_export]
    macro_rules! check_unsupported_field {
        ($object:ident) => {
            if $crate::config::get().amz_error_on_unsupported
                && let Some(field) = $object.unsupported_field()
            {
                return Ok($crate::server::err::builder::unsupported_param(field));
            }
        };
    }
}

#[macro_export]
macro_rules! deserialize_request_body {
    ($req:ident, $object:ty) => {{
        use $crate::{check_unsupported_field, deserialize_request_body_inner};

        let result = deserialize_request_body_inner!($req, $object);

        check_unsupported_field!(result);

        result
    }};
}
