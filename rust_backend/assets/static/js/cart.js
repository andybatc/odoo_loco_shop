document.addEventListener('DOMContentLoaded', () => {
    const { createApp } = Vue;

    createApp({
        delimiters: ['[[', ']]'],
        data() {
            const el = document.getElementById('cart-app');
            const rawItems = el ? el.getAttribute('data-items') : '[]';
            const rawTotal = el ? el.getAttribute('data-total') : '0';

            return {
                items: JSON.parse(rawItems),
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
                event.target.src = '/static/images/No image avaible.jpeg';
            }
        },
        mounted() {
            console.log("🛒 App del Carrito montada con éxito. Ítems cargados:", this.items.length);
        }
    }).mount('#cart-app');
});