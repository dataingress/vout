use std::borrow::Cow;

pub const INTERNAL_SERVICE_ERROR: &'static str = "InternalServiceError";
pub const INVALID_NEXT_TOKEN_EXCEPTION: &'static str = "InvalidNextTokenException";
pub const INVALID_PARAMETER_EXCEPTION: &'static str = "InvalidParameterException";
pub const RESOURCE_EXISTS_EXCEPTION: &'static str = "ResourceExistsException";
pub const RESOURCE_NOT_FOUND_EXCEPTION: &'static str = "ResourceNotFoundException";
pub const UNKNOWN_ROUTE: &'static str = "UnknownRoute";

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ErrorResponse<'a> {
    __type: &'static str,
    #[serde(rename = "Message")]
    message: Cow<'a, str>,
}

impl<'a> ErrorResponse<'a> {
    pub fn new(error: &'static str, message: Cow<'a, str>) -> anyhow::Result<String> {
        Ok(serde_json::to_string(&ErrorResponse {
            __type: error,
            message: message,
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
        invalid_parameter,
        super::INVALID_PARAMETER_EXCEPTION,
        "{}",
        message: impl std::fmt::Display
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
