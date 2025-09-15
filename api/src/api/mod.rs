// API modules will be added here by the generate command
// Example:
// pub mod users;
// pub use users::get_routes as users_routes;

pub mod stream;
pub use stream::get_routes as stream_routes;
pub mod client;
pub use client::get_routes as client_routes;
pub mod info;
pub use info::get_routes as info_routes;
