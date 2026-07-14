use aide::OperationInput;
use axum::extract::FromRequestParts;
use schemars::JsonSchema;

use crate::error::AppError;

/// Extractor for query
#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Query), rejection(AppError))]
pub struct Query<T>(pub T);

impl<T> OperationInput for Query<T>
where
    T: JsonSchema,
{
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        axum::extract::Query::<T>::operation_input(ctx, operation);
    }
}
