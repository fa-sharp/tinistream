use aide::OperationInput;
use axum::extract::FromRequest;
use schemars::JsonSchema;

use crate::error::AppError;

/// Extractor for a JSON body
#[derive(FromRequest)]
#[from_request(via(axum::extract::Json), rejection(AppError))]
pub struct JsonBody<T>(pub T);

impl<T> OperationInput for JsonBody<T>
where
    T: JsonSchema,
{
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        axum::extract::Json::<T>::operation_input(ctx, operation);
    }
}
