//! Validates JSON body + field rules before controller runs.
//! Express equivalent: `validateBody(schema)` middleware.

use axum::{
    extract::{FromRequest, Json, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;
use validator::{Validate, ValidationErrors};

use crate::error::{ApiResponse, AppError};

pub struct ValidatedJson<T>(pub T);

impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(payload) = Json::<T>::from_request(req, state)
            .await
            .map_err(json_error_response)?;

        payload
            .validate()
            .map_err(|errors| validation_error_response(errors))?;

        Ok(ValidatedJson(payload))
    }
}

fn json_error_response(err: axum::extract::rejection::JsonRejection) -> Response {
    AppError::BadRequest(format!("invalid json: {err}"))
        .into_response()
}

fn validation_error_response(errors: ValidationErrors) -> Response {
    let message = errors
        .field_errors()
        .iter()
        .flat_map(|(field, field_errors)| {
            field_errors.iter().map(move |error| {
                let msg = error
                    .message
                    .as_ref()
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| "invalid value".into());
                format!("{field}: {msg}")
            })
        })
        .collect::<Vec<_>>()
        .join(", ");

    (
        StatusCode::BAD_REQUEST,
        axum::Json(ApiResponse::<()> {
            success: false,
            data: None,
            message: Some(message),
        }),
    )
        .into_response()
}
