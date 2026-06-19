use odoo_shop::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;
use sea_orm::{ActiveValue::Set, ActiveModelTrait};
use odoo_shop::models::_entities::products;

#[tokio::test]
#[serial]
async fn can_search_products_by_name() {
    request::<App, _, _>(|request, ctx| async move {
        products::ActiveModel {
            odoo_id: Set(Some(1001)),
            name: Set(Some("Laptop Gamer RGB".to_string())),
            sku: Set(Some("LAP-001".to_string())),
            price: Set(Some(sea_orm::prelude::Decimal::new(1500, 0))),
            is_published: Set(true),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await
        .unwrap();

        products::ActiveModel {
            odoo_id: Set(Some(1002)),
            name: Set(Some("Mouse Inalámbrico".to_string())),
            sku: Set(Some("MOU-001".to_string())),
            price: Set(Some(sea_orm::prelude::Decimal::new(50, 0))),
            is_published: Set(true),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await
        .unwrap();

        let res = request.get("/shop/api/search?q=laptop").await;
        assert_eq!(res.status_code(), 200);

        let body: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        let items = body.as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["name"], "Laptop Gamer RGB");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_search_products_by_sku() {
    request::<App, _, _>(|request, ctx| async move {
        products::ActiveModel {
            odoo_id: Set(Some(2001)),
            name: Set(Some("Teclado Mecánico".to_string())),
            sku: Set(Some("TEC-2024".to_string())),
            price: Set(Some(sea_orm::prelude::Decimal::new(120, 0))),
            is_published: Set(true),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await
        .unwrap();

        let res = request.get("/shop/api/search?q=TEC-2024").await;
        assert_eq!(res.status_code(), 200);

        let body: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        let items = body.as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["name"], "Teclado Mecánico");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn search_empty_query_returns_empty() {
    request::<App, _, _>(|request, ctx| async move {
        products::ActiveModel {
            odoo_id: Set(Some(3001)),
            name: Set(Some("Monitor 4K".to_string())),
            is_published: Set(true),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await
        .unwrap();

        let res = request.get("/shop/api/search?q=").await;
        assert_eq!(res.status_code(), 200);

        let body: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        let items = body.as_array().unwrap();
        assert!(items.is_empty());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn search_no_results_returns_empty() {
    request::<App, _, _>(|request, ctx| async move {
        products::ActiveModel {
            odoo_id: Set(Some(4001)),
            name: Set(Some("Tablet".to_string())),
            is_published: Set(true),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await
        .unwrap();

        let res = request.get("/shop/api/search?q=zzzznotfound").await;
        assert_eq!(res.status_code(), 200);

        let body: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        let items = body.as_array().unwrap();
        assert!(items.is_empty());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn search_html_page_renders() {
    request::<App, _, _>(|request, ctx| async move {
        products::ActiveModel {
            odoo_id: Set(Some(5001)),
            name: Set(Some("Auriculares Bluetooth".to_string())),
            is_published: Set(true),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await
        .unwrap();

        let res = request.get("/shop/search?q=auriculares").await;
        assert_eq!(res.status_code(), 200);

        let text = res.text();
        assert!(text.contains("Auriculares Bluetooth"));
        assert!(text.contains("1 resultado"));
    })
    .await;
}
