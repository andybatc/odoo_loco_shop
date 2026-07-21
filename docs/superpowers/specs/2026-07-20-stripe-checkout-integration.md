# Stripe Checkout Integration

**Date:** 2026-07-20

## Summary

Add Stripe Checkout (hosted payment page) as a payment option in the existing checkout flow. When the user selects Stripe as payment method, they're redirected to Stripe's hosted page to pay. After payment, Stripe redirects back and the order is created in both the local DB and Odoo.

## Architecture

```
Frontend (Vue checkout)
  → POST /api/checkout/stripe-session (if method code == "stripe")
  → Stripe Checkout URL → redirect
  → User pays on Stripe
  → Redirect to /order/success?session_id=xxx
  → Verify session, create order, show confirmation
```

## Changes

### 1. Backend: New dependency

Add `stripe` crate to `Cargo.toml` for creating Checkout Sessions and verifying payments.

### 2. Backend: Config key

Store Stripe secret key in `configs` table as `stripe_secret_key` (configurable via admin UI, same pattern as `webhook_token`).

### 3. Backend: `POST /api/checkout/stripe-session`

**Input:** Same `CheckoutRequest` (customer info + payment_method_id)  
**Logic:**
- Validates cart and calculates total (reuse logic from `submit_checkout`)
- Creates Stripe `CheckoutSession` (mode: payment, line_items from cart)
- `success_url`: `{base_url}/order/success?session_id={CHECKOUT_SESSION_ID}`
- `cancel_url`: `{base_url}/checkout`
- Stores `{cart_uuid, customer_info, payment_method_id, user_id}` in Redis under `stripe:session:{session_id}` (TTL: 1 hour)

**Response:** `{ url: "https://checkout.stripe.com/c/pay_..." }`

### 4. Backend: Modified `/order/success`

Accept `?session_id=xxx` parameter:
- Fetch Stripe session, verify `payment_status == "paid"`
- Retrieve checkout data from Redis using session_id
- Create order in DB + order_items + dispatch OrderCreationWorker (same logic as `submit_checkout`)
- Clear cart
- Show confirmation with order details

If no `session_id` param: works as today (show from query params `ref`, `inv`, `total`).

### 5. Frontend (Vue checkout)

On submit:
- If selected payment method has `code == "stripe"` → call `/api/checkout/stripe-session` instead of `/api/checkout`
- On response: `window.location.href = response.url`
- Otherwise: behave as today

### 6. Admin: stripe_secret_key config

Add field in the config UI (`views.rs:config_page`) to set/get `stripe_secret_key`, same pattern as `webhook_token`.

## Data Storage

Redis key: `stripe:session:{session_id}` → JSON
```json
{
  "cart_uuid": "uuid",
  "customer": { "name": "...", "email": "...", ... },
  "payment_method_id": 123,
  "user_id": null
}
```
TTL: 3600s (Stripe sessions expire at 1h by default)

## Out of Scope (YAGNI)

- Stripe webhook (localhost unreachable, success page handles confirmation)
- 3D Secure handling (Stripe handles automatically)
- Saved cards / tokenization
- Refunds
- Multi-currency

## Sequence

```
User → checkout.html (selects Stripe as method)
User → clicks "Pagar"
Frontend → POST /api/checkout/stripe-session { customer, payment_method_id }
Backend → creates Stripe CheckoutSession, stores data in Redis
Backend → returns { url }
Frontend → window.location.href = url
User → pays on Stripe
Stripe → redirects to /order/success?session_id=xxx
Backend → verifies session, creates order, dispatches Odoo sync
Backend → renders order_success.html with order details
```
