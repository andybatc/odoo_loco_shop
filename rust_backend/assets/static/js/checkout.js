function parseJsonScript(id) {
    const el = document.getElementById(id);
    if (!el) return null;
    try { return JSON.parse(el.textContent); } catch (e) { return null; }
}

const app = Vue.createApp({
    delimiters: ['[[', ']]'],
    data() {
        const el = document.getElementById('checkout-app');
        const userData = parseJsonScript('checkout-user-data') || {};
        return {
            items: parseJsonScript('checkout-items') || [],
            totalGeneral: parseFloat(el?.getAttribute('data-total') || '0'),
            paymentMethods: parseJsonScript('checkout-payment-methods') || [],
            countries: parseJsonScript('checkout-countries') || {},
            shippingCost: null,
            shippingOrigin: '',
            selectedPaymentId: null,
            customer: {
                name: userData.name || '',
                email: userData.email || '',
                phone: userData.phone || '',
                street: userData.street || '',
                city: userData.city || '',
                zip: userData.zip || '',
                country: userData.country || '',
                state: userData.state || '',
            },
            userData: Object.keys(userData).length > 0 ? userData : null,
            submitting: false,
            errorMessage: '',
        };
    },
    computed: {
        productIds() {
            return this.items.map(i => i.id).join(',');
        },
        totalWithShipping() {
            if (this.noShipping) return this.totalGeneral.toFixed(2);
            const base = this.totalGeneral;
            const ship = this.shippingCost !== null ? parseFloat(this.shippingCost) : 0;
            return (base + ship).toFixed(2);
        },
        noShipping() {
            return this.shippingCost === 0 && (!this.shippingOrigin || this.shippingOrigin.includes('Sin origen'));
        },
    },
    watch: {
        'customer.country': function (newVal, oldVal) {
            if (newVal !== oldVal) this.customer.state = '';
            this.estimateShipping();
        },
        'customer.state': function () { this.estimateShipping(); },
    },
    methods: {
        async estimateShipping() {
            const country = this.customer.country;
            const state = this.customer.state;
            if (!country || !state || !this.productIds) {
                this.shippingCost = null;
                this.shippingOrigin = '';
                return;
            }
            try {
                const resp = await fetch(`/api/shipping/estimate?product_ids=${this.productIds}&country=${encodeURIComponent(country)}&state=${encodeURIComponent(state)}`);
                const data = await resp.json();
                this.shippingCost = parseFloat(data.shipping_cost) || 0;
                this.shippingOrigin = data.origin_summary || '';
            } catch {
                this.shippingCost = null;
                this.shippingOrigin = '';
            }
        },
        async submitOrder() {
            this.submitting = true;
            this.errorMessage = '';

            try {
                const body = { customer: this.customer };
                if (this.selectedPaymentId) {
                    body.payment_method_id = this.selectedPaymentId;
                }
                const response = await fetch('/api/checkout', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(body),
                });

                const data = await response.json();

                if (data.success) {
                    const params = new URLSearchParams({
                        ref: data.order_name || '',
                        inv: data.invoice_name || '',
                        total: this.totalGeneral.toFixed(2),
                    });
                    window.location.href = '/order/success?' + params.toString();
                } else {
                    this.errorMessage = data.error || 'Error al procesar el pedido';
                }
            } catch (err) {
                this.errorMessage = 'Error de conexión con el servidor';
            } finally {
                this.submitting = false;
            }
        },
    },
});

app.mount('#checkout-app');
