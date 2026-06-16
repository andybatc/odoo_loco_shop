const app = Vue.createApp({
    delimiters: ['[[', ']]'],
    data() {
        const el = document.getElementById('checkout-app');
        return {
            items: JSON.parse(el.dataset.items || '[]'),
            totalGeneral: parseFloat(el.dataset.total || '0'),
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
                const response = await fetch('/api/checkout', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ customer: this.customer }),
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
