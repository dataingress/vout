use std::borrow::Cow;

pub const INTERNAL_SERVICE_ERROR: &str = "InternalServiceError";
pub const INVALID_ALLOWED_PATTERN_EXCEPTION: &str = "InvalidAllowedPatternException";
pub const INVALID_FILTER_KEY: &str = "InvalidFilterKey";
pub const INVALID_FILTER_OPTION: &str = "InvalidFilterOption";
pub const INVALID_FILTER_VALUE: &str = "InvalidFilterValue";
pub const INVALID_NEXT_TOKEN_EXCEPTION: &str = "InvalidNextTokenException";
pub const INVALID_PARAMETER_EXCEPTION: &str = "InvalidParameterException";
pub const PARAMETER_ALREADY_EXISTS: &str = "ParameterAlreadyExists";
pub const PARAMETER_NOT_FOUND: &str = "ParameterNotFound";
pub const PARAMETER_VERSION_LABEL_LIMIT_EXCEEDED: &str = "ParameterVersionLabelLimitExceeded";
pub const PARAMETER_VERSION_NOT_FOUND: &str = "ParameterVersionNotFound";
pub const RESOURCE_EXISTS_EXCEPTION: &str = "ResourceExistsException";
pub const RESOURCE_NOT_FOUND_EXCEPTION: &str = "ResourceNotFoundException";
pub const UNKNOWN_ROUTE: &str = "UnknownRoute";

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ErrorResponse<'a> {
    __type: &'static str,
    #[serde(rename = "Message")]
    message: Cow<'a, str>,
}

impl<'a> ErrorResponse<'a> {
    pub fn render(error: &'static str, message: Cow<'a, str>) -> anyhow::Result<String> {
        Ok(serde_json::to_string(&ErrorResponse {
            __type: error,
            message,
        })?)
    }
}

pub mod builder {
    use crate::server::res::GwPassed;
    use std::borrow::Cow;

    macro_rules! build_response {
        ($func:ident, $error:expr, $message:expr, $($arg:ident : $ty:ty),*) => {
            pub fn $func<'a>($($arg: $ty),*) -> GwPassed<'a> {
                GwPassed::Failure(($error, Cow::Owned(format!($message, $($arg),*))))
            }
        };
    }

    build_response!(unknown_route, super::UNKNOWN_ROUTE, "Unknown route.",);

    build_response!(
        internal_service_error,
        super::INTERNAL_SERVICE_ERROR,
        "An internal service error occurred.",
    );

    build_response!(
        invalid_request_body,
        super::INVALID_PARAMETER_EXCEPTION,
        "Invalid request body.",
    );

    build_response!(
        invalid_next_token,
        super::INVALID_NEXT_TOKEN_EXCEPTION,
        "The NextToken value is invalid.",
    );

    build_response!(
        invalid_filter_key,
        super::INVALID_FILTER_KEY,
        "The specified filter key is not valid.",
    );

    build_response!(
        invalid_filter_option,
        super::INVALID_FILTER_OPTION,
        "The specified filter option is not valid.",
    );

    build_response!(
        invalid_filter_value,
        super::INVALID_FILTER_VALUE,
        "The specified filter value is not valid.",
    );

    build_response!(
        invalid_parameter,
        super::INVALID_PARAMETER_EXCEPTION,
        "{}",
        message: impl std::fmt::Display
    );

    build_response!(
        invalid_allowed_pattern,
        super::INVALID_ALLOWED_PATTERN_EXCEPTION,
        "The request doesn't meet the regular expression requirement.",
    );

    build_response!(
        parameter_already_exists,
        super::PARAMETER_ALREADY_EXISTS,
        "The parameter already exists. To overwrite this value, set the overwrite option in the request to true.",
    );

    build_response!(
        parameter_not_found,
        super::PARAMETER_NOT_FOUND,
        "The parameter couldn't be found. Verify the name and try again.",
    );

    build_response!(
        parameter_version_not_found,
        super::PARAMETER_VERSION_NOT_FOUND,
        "The specified parameter version wasn't found. Verify the parameter name and version, and try again.",
    );

    build_response!(
        parameter_version_label_limit_exceeded,
        super::PARAMETER_VERSION_LABEL_LIMIT_EXCEEDED,
        "A parameter version can have a maximum of ten labels.",
    );

    build_response!(
        unsupported_param,
        super::INVALID_PARAMETER_EXCEPTION,
        "Unsupported request field '{}'. If you need it, please use the official AWS Secrets Manager API instead.",
        param1: impl std::fmt::Display
    );

    build_response!(
        resource_exists,
        super::RESOURCE_EXISTS_EXCEPTION,
        "The operation failed because the secret {} already exists.",
        name: impl std::fmt::Display
    );

    build_response!(
        resource_not_found,
        super::RESOURCE_NOT_FOUND_EXCEPTION,
        "Secrets Manager can't find the specified secret.",
    );
}
