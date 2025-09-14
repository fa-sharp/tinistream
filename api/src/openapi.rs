use rocket_okapi::{
    rapidoc::{make_rapidoc, GeneralConfig, Layout, LayoutConfig, RapiDocConfig, RenderStyle},
    settings::UrlObject,
};

/// Create the OpenAPI doc routes
pub fn get_openapi_routes() -> impl Into<Vec<rocket::Route>> {
    make_rapidoc(&RapiDocConfig {
        title: Some(String::from("RsStreamer API Documentation")),
        general: GeneralConfig {
            heading_text: String::from("RsStreamer API"),
            spec_urls: vec![UrlObject::new("OpenAPI Schema", "/api/openapi.json")],
            ..Default::default()
        },
        layout: LayoutConfig {
            layout: Layout::Column,
            render_style: RenderStyle::View,
            ..Default::default()
        },
        ..Default::default()
    })
}
