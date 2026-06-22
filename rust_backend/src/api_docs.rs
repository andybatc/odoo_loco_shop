use utoipa::OpenApi;

use crate::controllers::{
    auth, carts, checkout, config, shop, products_webhook,
};
use crate::models::users::{LoginParams, RegisterParams};
use crate::views::auth::{CurrentResponse, LoginResponse};
use crate::workers::webhook::WebhookWorkerArgs;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Odoo Loco Shop API",
        version = "1.0.0",
        description = "API REST para la tienda online integrada con Odoo 18.0"
    ),
    servers(
        (url = "http://localhost:5150", description = "Development server")
    ),
    paths(
        auth::login,
        auth::register,
        auth::current,
        config::get_token,
        config::update_token,
        config::get_odoo_url,
        config::update_odoo_url,
        carts::get_cart_items,
        carts::update_cart_item_quantity,
        carts::remove_cart_item,
        shop::search_api,
        shop::get_product,
        checkout::submit_checkout,
        products_webhook::update,
        products_webhook::update_bulk,
    ),
    components(schemas(
        LoginParams,
        RegisterParams,
        LoginResponse,
        CurrentResponse,
        config::TokenRequest,
        config::OdooUrlRequest,
        shop::ProductDetail,
        shop::SearchResultItem,
        carts::CartItemWithProduct,
        carts::UpdateQuantityParams,
        checkout::CheckoutRequest,
        checkout::CheckoutResponse,
        checkout::CustomerInfo,
        WebhookWorkerArgs,
    ))
)]
pub struct ApiDoc;
