# Stripe Checkout Integration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Integrate Stripe Checkout (hosted) into the existing shop checkout flow so customers can pay with real credit cards.

**Architecture:** When user selects Stripe as payment method, frontend calls `POST /api/checkout/stripe-session` instead of the regular checkout endpoint. Backend creates a Stripe CheckoutSession, stores checkout data in Redis, returns the Stripe URL for redirect. After payment, Stripe redirects to `/order/success?session_id=cs_test_xxx` where the backend verifies payment, creates the order (same logic as `submit_checkout`), and shows confirmation.

**Tech Stack:** async-stripe 1.0.0-rc.6 (crate `async-stripe` as `stripe`), Redis cache (existing), Loco.rs routes, SeaORM

## Global Constraints

- Use `async-stripe = "=1.0.0-rc.6"` pinned exact version.
- All amounts sent to Stripe in cents (multiply by 100).
- Currency: MXN (Mexican pesos).
- Store checkout data in Redis under key `stripe:session:{session_id}` with 3600s TTL.
- Follow existing code patterns: `use loco_rs::prelude::*`, `#[debug_handler]`, `State(ctx): State<AppContext>`.
- Stripe secret key stored in `configs` table as `stripe_secret_key` (same pattern as `webhook_token`).
- Do NOT add webhook endpoint — success page verifies synchronously.
- Frontend detects Stripe by checking `pm.code === 'stripe'` on the payment method.

---
## Task 1: Add stripe dependency + stripe_secret_key config management

**Files:**
- Modify: `rust_backend/Cargo.toml`
- Modify: `rust_backend/src/controllers/views.rs` (ConfigUpdateForm struct, config_page handler, handle_config_update handler)
- Modify: `rust_backend/assets/views/config/ui.html`

**Produced:** Stripe secret key stored in `configs` table, loadable via `config_cache::get_cached_config(&ctx, "stripe_secret_key")`.

- [ ] **Step 1: Add async-stripe crate to Cargo.toml**

Add to `[dependencies]`:
```toml
async-stripe = { version = "=1.0.0-rc.6", default-features = false, features = ["runtime-tokio-hyper-rustls"] }
```

- [ ] **Step 2: Add stripe_secret_key to ConfigUpdateForm**

In `views.rs`, add to `ConfigUpdateForm` struct:
```rust
pub stripe_secret_key: Option<String>,
```

- [ ] **Step 3: Load stripe_secret_key in config_page handler**

After the `shipping_local_rate` block in `config_page`, add:
```rust
let stripe_secret_key = config_cache::get_cached_config(&ctx, "stripe_secret_key")
    .await
    .map_err(|e| {
        tracing::error!("Error consultando cache: {:?}", e);
        Error::string("Error al conectar con la base de datos")
    })?
    .unwrap_or_else(|| "No configurado".to_string());
```
Add to template context:
```rust
"stripe_secret_key": stripe_secret_key,
```

- [ ] **Step 4: Handle stripe_secret_key update in handle_config_update**

After the `shipping_local_rate` if-block, add:
```rust
if let Some(ref key) = payload.stripe_secret_key {
    if !key.is_empty() && key.len() < 8 {
        return Err(Error::BadRequest("stripe_secret_key must be at least 8 characters".to_string()));
    }
    if !key.is_empty() {
        let config = configs::Entity::find()
            .filter(configs::Column::Key.eq("stripe_secret_key"))
            .one(&ctx.db)
            .await?;
        if let Some(c) = config {
            let mut active_model: configs::ActiveModel = c.into();
            active_model.value = Set(Some(key.clone()));
            active_model.update(&ctx.db).await?;
        } else {
            configs::ActiveModel {
                key: Set(Some("stripe_secret_key".to_string())),
                value: Set(Some(key.clone())),
                ..Default::default()
            }.insert(&ctx.db).await?;
        }
        config_cache::invalidate_config_cache(&ctx, "stripe_secret_key").await;
    }
}
```

- [ ] **Step 5: Add Stripe section to config UI template**

Add before the closing `</div>` of the container in `ui.html`:
```html
<hr class="border-gray-200">
<div class="bg-red-50 rounded-md p-4">
    <h3 class="text-sm font-medium text-red-800">Clave Secreta Stripe</h3>
    <div class="mt-2 text-sm text-red-700 font-mono break-all bg-white p-2 rounded border border-red-200">
        {{ stripe_secret_key }}
    </div>
</div>
<form hx-post="/ui/auth/config" hx-target="#msg-stripe" class="space-y-4">
    <div>
        <label for="stripe_secret_key" class="block text-sm font-medium text-gray-700">Nueva clave secreta Stripe</label>
        <p class="text-xs text-gray-500">sk_test_xxx o sk_live_xxx</p>
        <input id="stripe_secret_key" name="stripe_secret_key" type="text" value="{{ stripe_secret_key }}"
               class="appearance-none block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm placeholder-gray-400 focus:outline-none focus:ring-red-500 focus:border-red-500 sm:text-sm">
    </div>
    <div>
        <button type="submit"
                class="w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-red-600 hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500">
            Actualizar Clave Stripe
        </button>
    </div>
    <div id="msg-stripe" class="text-center text-sm"></div>
</form>
```

---
## Task 2: POST /api/checkout/stripe-session endpoint

**Files:**
- Modify: `rust_backend/src/controllers/checkout.rs`

**Produced:** New API endpoint that creates a Stripe CheckoutSession and returns the redirect URL.

- [ ] **Step 1: Add structs and imports**

At the end of existing imports, add `use std::collections::HashMap;`.

Before `pub fn routes()`, add:
```rust
#[derive(Serialize)]
pub struct StripeSessionResponse {
    pub success: bool,
    pub url: Option<String>,
    pub error: Option<String>,
}
```

- [ ] **Step 2: Add create_stripe_session handler**

After `submit_checkout` and before `order_success`:
```rust
#[debug_handler]
pub(crate) async fn create_stripe_session(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(params): Json<CheckoutRequest>,
) -> Result<Json<StripeSessionResponse>> {
    // Validate email
    let email_re = regex::Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap();
    if !email_re.is_match(&params.customer.email) {
        return Ok(Json(StripeSessionResponse {
            success: false, url: None, error: Some("Email inválido".to_string()),
        }));
    }
    if params.customer.name.trim().is_empty() {
        return Ok(Json(StripeSessionResponse {
            success: false, url: None, error: Some("El nombre es obligatorio".to_string()),
        }));
    }

    // Get cart (same as submit_checkout)
    let cart_uuid = {
        let user = get_current_user(&ctx, headers.get("cookie").and_then(|h| h.to_str().ok()).map(|s| s.to_string())).await;
        if let Some(ref u) = user {
            let cart = carts::Entity::find()
                .filter(carts::Column::UserId.eq(u.pid))
                .one(&ctx.db).await?;
            match cart {
                Some(c) => c.id,
                None => return Ok(Json(StripeSessionResponse {
                    success: false, url: None, error: Some("Carrito no encontrado".to_string()),
                })),
            }
        } else {
            let cookie_val = headers.get("cookie")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.split(';').find(|p| p.trim().starts_with("rsv_cart_session=")))
                .and_then(|s| s.split('=').nth(1).map(|v| v.trim().to_string()));
            match cookie_val.and_then(|v| Uuid::parse_str(&v).ok()) {
                Some(id) => id,
                None => return Ok(Json(StripeSessionResponse {
                    success: false, url: None, error: Some("Carrito no encontrado".to_string()),
                })),
            }
        }
    };

    // Load cart items
    let items = cart_items::Entity::find()
        .filter(cart_items::Column::CartId.eq(cart_uuid))
        .all(&ctx.db).await?;
    if items.is_empty() {
        return Ok(Json(StripeSessionResponse {
            success: false, url: None, error: Some("El carrito está vacío".to_string()),
        }));
    }

    let mut product_ids = Vec::new();
    let mut item_map = HashMap::new();
    for item in &items {
        product_ids.push(item.product_id);
        item_map.insert(item.product_id, item.quantity);
    }

    let db_products = products::Entity::find()
        .filter(products::Column::Id.is_in(product_ids))
        .all(&ctx.db).await?;

    // Build Stripe line items (price_data with inline product)
    let mut stripe_line_items = Vec::new();
    for prod in &db_products {
        let qty = *item_map.get(&prod.id).unwrap_or(&1) as u64;
        let price_cents = prod.price
            .map(|p| (p.to_string().parse::<f64>().unwrap_or(0.0) * 100.0).round() as i64)
            .unwrap_or(0);
        let name = prod.name.clone().unwrap_or_else(|| "Producto".to_string());

        stripe_line_items.push(stripe::CreateCheckoutSessionLineItems {
            quantity: Some(qty),
            price_data: Some(stripe::CreateCheckoutSessionLineItemsPriceData {
                currency: stripe::Currency::MXN,
                product_data: Some(stripe::CreateCheckoutSessionLineItemsPriceDataProductData {
                    name: Some(&name),
                    ..Default::default()
                }),
                unit_amount: Some(price_cents),
                ..Default::default()
            }),
            ..Default::default()
        });
    }

    // Get Stripe secret key
    let secret_key = crate::models::config_cache::get_cached_config(&ctx, "stripe_secret_key")
        .await?.unwrap_or_default();
    if secret_key.is_empty() || secret_key == "No configurado" {
        return Ok(Json(StripeSessionResponse {
            success: false, url: None, error: Some("Stripe no configurado".to_string()),
        }));
    }

    // Create Stripe CheckoutSession
    let client = stripe::Client::new(&secret_key);
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("cart_uuid".to_string(), cart_uuid.to_string());
    metadata.insert("payment_method_id".to_string(), params.payment_method_id.unwrap_or(0).to_string());

    let base_url = crate::models::config_cache::get_cached_config(&ctx, "odoo_base_url")
        .await?.unwrap_or_else(|| "http://localhost:5150".to_string());

    let session_params = stripe::CreateCheckoutSession {
        mode: Some(stripe::CheckoutSessionMode::Payment),
        success_url: Some(&format!("{}/order/success?session_id={{CHECKOUT_SESSION_ID}}", base_url)),
        cancel_url: Some(&format!("{}/checkout", base_url)),
        line_items: Some(stripe_line_items),
        customer_email: Some(&params.customer.email),
        metadata: Some(metadata),
        ..Default::default()
    };

    let session = stripe::CheckoutSession::create(&client, session_params).await.map_err(|e| {
        tracing::error!("Stripe session creation error: {:?}", e);
        Error::string(format!("Error al crear sesión de pago: {}", e))
    })?;

    let session_url = session.url.ok_or_else(|| {
        tracing::error!("Stripe session created without URL");
        Error::string("Error al crear sesión de pago")
    })?;

    // Store checkout data in Redis for the callback
    let checkout_data = serde_json::json!({
        "cart_uuid": cart_uuid,
        "customer": {
            "name": params.customer.name,
            "email": params.customer.email,
            "phone": params.customer.phone,
            "street": params.customer.street,
            "city": params.customer.city,
            "zip": params.customer.zip,
            "country": params.customer.country,
            "state": params.customer.state,
        },
        "payment_method_id": params.payment_method_id,
    });
    let _ = ctx.cache.insert_with_expiry(
        &format!("stripe:session:{}", session.id.as_ref()),
        &checkout_data,
        std::time::Duration::from_secs(3600),
    ).await;

    // Store shipping cost separately
    let dest_country = params.customer.country.as_deref().unwrap_or("");
    let dest_state = params.customer.state.as_deref().unwrap_or("");
    let (shipping_cost, _) = calc_shipping(&ctx.db, &db_products, dest_country, dest_state).await?;
    let _ = ctx.cache.insert_with_expiry(
        &format!("stripe:shipping:{}", session.id.as_ref()),
        &serde_json::json!({ "cost": shipping_cost.to_string() }),
        std::time::Duration::from_secs(3600),
    ).await;

    Ok(Json(StripeSessionResponse {
        success: true,
        url: Some(session_url),
        error: None,
    }))
}
```

- [ ] **Step 3: Register the route**

In `routes()` function, add after `post(submit_checkout)`:
```rust
.add("/api/checkout/stripe-session", post(create_stripe_session))
```

---
## Task 3: Modify order_success for Stripe callback

**Files:**
- Modify: `rust_backend/src/controllers/checkout.rs` (order_success function)

**Produced:** Modified success page that handles Stripe callback (verifies session, creates order, clears cart).

- [ ] **Step 1: Add import**

Add `use std::str::FromStr;` at the top of `checkout.rs`.

- [ ] **Step 2: Replace order_success function**

Replace the existing `order_success` with this version that handles both flows:

```rust
pub async fn order_success(
    ViewEngine(v): ViewEngine<TeraView>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    headers: HeaderMap,
    State(ctx): State<AppContext>,
    jar: CookieJar,
) -> Result<(CookieJar, Response)> {
    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));
    let user = get_current_user(&ctx, cookie_header).await;

    // Stripe callback flow
    if let Some(session_id) = params.get("session_id") {
        // 1. Verify the Stripe session
        let secret_key = crate::models::config_cache::get_cached_config(&ctx, "stripe_secret_key")
            .await?.unwrap_or_default();
        if secret_key.is_empty() || secret_key == "No configurado" {
            return Err(Error::string("Stripe no configurado"));
        }

        let client = stripe::Client::new(&secret_key);
        let sid = stripe::CheckoutSessionId::from_str(session_id)
            .map_err(|_| Error::BadRequest("ID de sesión inválido".to_string()))?;

        let session = stripe::CheckoutSession::retrieve(&client, &sid, &[]).await.map_err(|e| {
            tracing::error!("Stripe session retrieve error: {:?}", e);
            Error::string("Error al verificar el pago")
        })?;

        if session.payment_status != stripe::CheckoutSessionPaymentStatus::Paid {
            return Err(Error::BadRequest("El pago no fue completado".to_string()));
        }

        // 2. Retrieve checkout data from Redis
        let redis_key = format!("stripe:session:{}", session_id);
        let checkout_data: Option<serde_json::Value> = ctx.cache.get(&redis_key).await.ok().flatten();
        let checkout_data = checkout_data.ok_or_else(|| {
            Error::BadRequest("Sesión expirada. Por favor, intente nuevamente.".to_string())
        })?;

        let cart_uuid_str = checkout_data["cart_uuid"].as_str().unwrap_or("");
        let cart_uuid = Uuid::parse_str(cart_uuid_str)
            .map_err(|_| Error::BadRequest("Datos de sesión inválidos".to_string()))?;

        // 3. Check if already processed (cart already cleared)
        let product_items = cart_items::Entity::find()
            .filter(cart_items::Column::CartId.eq(cart_uuid))
            .all(&ctx.db).await?;

        if product_items.is_empty() {
            // Already processed — show existing order data
            return Ok((jar, format::render().view(
                &v, "shop/order_success.html",
                serde_json::json!({
                    "order_ref": session_id,
                    "invoice_ref": "",
                    "total": "0.00",
                    "current_user": user,
                    "already_processed": true,
                }),
            )?));
        }

        // 4. Rebuild order data from stored checkout info
        let customer = &checkout_data["customer"];
        let mut pids = Vec::new();
        let mut item_qty = std::collections::HashMap::new();
        for item in &product_items {
            pids.push(item.product_id);
            item_qty.insert(item.product_id, item.quantity);
        }

        let db_products = products::Entity::find()
            .filter(products::Column::Id.is_in(pids))
            .all(&ctx.db).await?;

        // Recalculate total
        let shipping_key = format!("stripe:shipping:{}", session_id);
        let shipping_data: Option<serde_json::Value> = ctx.cache.get(&shipping_key).await.ok().flatten();
        let shipping_cost: sea_orm::prelude::Decimal = shipping_data
            .and_then(|v| v["cost"].as_str()?.parse::<f64>().ok())
            .map(|v| sea_orm::prelude::Decimal::try_from(v).unwrap_or_default())
            .unwrap_or_default();

        let mut total = shipping_cost;
        for prod in &db_products {
            let qty = *item_qty.get(&prod.id).unwrap_or(&1);
            let price = prod.price.unwrap_or_default();
            total += price * sea_orm::prelude::Decimal::from(qty as i64);
        }
        let total_f64 = total.to_string().parse::<f64>().unwrap_or(0.0);

        // 5. Create order
        let order_id = Uuid::new_v4();
        let order = orders_entity::ActiveModel {
            id: Set(order_id),
            user_id: Set(user.as_ref().map(|u| u.id)),
            customer_name: Set(customer["name"].as_str().unwrap_or("").to_string()),
            customer_email: Set(customer["email"].as_str().unwrap_or("").to_string()),
            customer_phone: Set(customer["phone"].as_str().map(|s| s.to_string())),
            customer_street: Set(customer["street"].as_str().map(|s| s.to_string())),
            customer_city: Set(customer["city"].as_str().map(|s| s.to_string())),
            customer_zip: Set(customer["zip"].as_str().map(|s| s.to_string())),
            customer_country: Set(customer["country"].as_str().map(|s| s.to_string())),
            customer_state: Set(customer["state"].as_str().map(|s| s.to_string())),
            shipping_cost: Set(Some(shipping_cost)),
            total: Set(total),
            status: Set("paid".to_string()),
            ..Default::default()
        };
        order.insert(&ctx.db).await?;

        // 6. Create order items
        let order_items_to_insert: Vec<order_items::ActiveModel> = db_products.iter().map(|prod| {
            let qty = *item_qty.get(&prod.id).unwrap_or(&1);
            let price = prod.price.unwrap_or_default();
            let subtotal = price * sea_orm::prelude::Decimal::from(qty as i64);
            order_items::ActiveModel {
                id: Set(Uuid::new_v4()),
                order_id: Set(order_id),
                product_id: Set(prod.id),
                product_name: Set(prod.name.clone().unwrap_or_else(|| "Product".to_string())),
                price: Set(price),
                quantity: Set(qty),
                subtotal: Set(subtotal),
                ..Default::default()
            }
        }).collect();
        order_items::Entity::insert_many(order_items_to_insert).exec(&ctx.db).await?;

        // 7. Dispatch Odoo sync
        let worker_args = crate::workers::order_creation::OrderWorkerArgs { order_id };
        crate::workers::order_creation::OrderCreationWorker::perform_later(&ctx, worker_args).await?;

        // 8. Clear cart
        cart_items::Entity::delete_many()
            .filter(cart_items::Column::CartId.eq(cart_uuid))
            .exec(&ctx.db).await?;
        carts::Entity::delete_by_id(cart_uuid).exec(&ctx.db).await?;

        // 9. Clean up Redis
        let _ = ctx.cache.delete(&redis_key).await;
        let _ = ctx.cache.delete(&shipping_key).await;

        // 10. Remove guest cart cookie
        let jar = jar.remove(Cookie::new("rsv_cart_session", ""));

        return Ok((jar, format::render().view(
            &v, "shop/order_success.html",
            serde_json::json!({
                "order_ref": order_id.to_string(),
                "invoice_ref": "",
                "total": format!("{:.2}", total_f64),
                "current_user": user,
                "already_processed": false,
            }),
        )?));
    }

    // Existing non-Stripe flow
    let order_ref = params.get("ref").cloned().unwrap_or_default();
    let invoice_ref = params.get("inv").cloned().unwrap_or_default();
    let total = params.get("total").cloned().unwrap_or_else(|| "0.00".to_string());

    let response = format::render().view(
        &v, "shop/order_success.html",
        serde_json::json!({
            "order_ref": order_ref,
            "invoice_ref": invoice_ref,
            "total": total,
            "current_user": user,
            "already_processed": false,
        }),
    )?;

    Ok((jar, response))
}
```

---
## Task 4: Frontend — detect Stripe method and redirect to Stripe

**Files:**
- Modify: `rust_backend/assets/static/js/checkout.js`

**Produced:** Modified checkout JS that detects Stripe payment method and calls the new endpoint.

- [ ] **Step 1: Modify submitOrder method in checkout.js**

Replace the `submitOrder` method in checkout.js. The new version:
1. Finds the selected payment method object in `paymentMethods` array
2. If its `code` is `"stripe"`, calls `/api/checkout/stripe-session` instead of `/api/checkout`
3. On success, redirects to `data.url` (the Stripe Checkout URL)
4. Otherwise, behaves exactly as today

```javascript
async submitOrder() {
    this.submitting = true;
    this.errorMessage = '';

    try {
        const body = { customer: this.customer };
        if (this.selectedPaymentId) {
            body.payment_method_id = this.selectedPaymentId;
        }

        // Check if selected method is Stripe
        const selectedMethod = this.paymentMethods.find(
            pm => pm.odoo_provider_id === this.selectedPaymentId
        );
        const isStripe = selectedMethod && selectedMethod.code === 'stripe';

        const endpoint = isStripe ? '/api/checkout/stripe-session' : '/api/checkout';
        const response = await fetch(endpoint, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(body),
        });

        const data = await response.json();

        if (data.success) {
            if (isStripe && data.url) {
                // Stripe: redirect directly to Stripe Checkout
                window.location.href = data.url;
            } else {
                // Non-Stripe: existing flow
                const params = new URLSearchParams({
                    ref: data.order_name || '',
                    inv: data.invoice_name || '',
                    total: this.totalGeneral.toFixed(2),
                });
                window.location.href = '/order/success?' + params.toString();
            }
        } else {
            this.errorMessage = data.error || 'Error al procesar el pedido';
        }
    } catch (err) {
        this.errorMessage = 'Error de conexión con el servidor';
    } finally {
        this.submitting = false;
    }
},
```

- [ ] **Step 2: Bump asset version for cache busting (optional but safe)**

In `checkout.html`, find the script tag and bump the version:
```html
<script src="/static/js/checkout.js?v=1.4"></script>
```

- [ ] **Step 3: Build Tailwind CSS**

```bash
cd rust_backend && npx @tailwindcss/cli -i assets/static/css/tailwind-input.css -o assets/static/css/tailwind.css --minify
```
