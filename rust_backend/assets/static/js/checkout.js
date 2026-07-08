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
    methods: {
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
