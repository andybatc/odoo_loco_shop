function parseCheckoutData() {
    let data = { items: [], total: 0, payment_methods: [] };
    const scriptEl = document.getElementById('checkout-data');
    if (scriptEl) {
        try {
            data = JSON.parse(scriptEl.textContent);
        } catch (e) {
            console.error('Error parsing checkout data:', e);
        }
    }
    return data;
}

const app = Vue.createApp({
    delimiters: ['[[', ']]'],
    data() {
        const raw = parseCheckoutData();
        return {
            items: raw.items || [],
            totalGeneral: raw.total || 0,
            paymentMethods: raw.payment_methods || [],
            selectedPaymentId: null,
            customer: {
                name: '',
                email: '',
                phone: '',
                street: '',
                city: '',
                zip: '',
            },
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
