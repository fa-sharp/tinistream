use rocket_okapi::{
    rapidoc::{make_rapidoc, GeneralConfig, Layout, LayoutConfig, RapiDocConfig, RenderStyle},
    settings::UrlObject,
};

/// Create the OpenAPI doc routes
pub fn get_openapi_routes() -> impl Into<Vec<rocket::Route>> {
    make_rapidoc(&RapiDocConfig {
        title: Some(String::from("Tinistream API Documentation")),
        general: GeneralConfig {
            heading_text: String::from("Tinistream API"),
            spec_urls: vec![UrlObject::new("OpenAPI Schema", "/openapi.json")],
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
