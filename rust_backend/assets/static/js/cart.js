document.addEventListener('DOMContentLoaded', () => {
    const { createApp } = Vue;

    createApp({
        delimiters: ['[[', ']]'],
        data() {
            const rawTotal = document.getElementById('cart-app')?.getAttribute('data-total') || '0';
            let items = [];
            const scriptEl = document.getElementById('cart-data');
            if (scriptEl) {
                try {
                    items = JSON.parse(scriptEl.textContent);
                } catch (e) {
                    console.error('Error parsing cart data:', e);
                    items = [];
                }
            }
            return {
                items,
                // Guardamos el total inicial enviado por el backend en Rust
                totalBackend: parseFloat(rawTotal)
            }
        },
        computed: {
            // Recalcular dinámicamente en el cliente por seguridad reactiva
            totalGeneral() {
                return this.items.reduce((acc, item) => {
                    return acc + (item.price * item.quantity);
                }, 0);
            }
        },
        methods: {
            handleImageError(event) {
                event.target.src = '/static/images/No image available.jpeg';
            }
        },
        mounted() {
            console.log("🛒 App del Carrito montada con éxito. Ítems cargados:", this.items.length);
        }
    }).mount('#cart-app');
});