document.addEventListener('DOMContentLoaded', () => {
    const {createApp} = Vue;

    createApp({
        delimiters: ['[[', ']]'],
        data() {
            const el = document.getElementById('shop-app');
            const rawData = el ? el.getAttribute('data-products') : '[]';

            return {
                products: JSON.parse(rawData),
                searchQuery: '',
                addingToCart: null
            }
        },
        computed: {
            filteredProducts() {
                if (!this.products) return [];

                const query = this.searchQuery.toLowerCase();
                return this.products.filter(product => {
                    const name = product.name ? product.name.toLowerCase() : '';
                    const desc = product.description ? product.description.toLowerCase() : '';
                    return name.includes(query) || desc.includes(query);
                });
            }
        },
        methods: {
            handleImageError(event) {
                event.target.src = '/static/images/No image avaible.jpeg';
            },
            async addToCart(productId) {
                this.addingToCart = productId;

                try {
                    const response = await fetch('/api/carts', {
                        method: 'POST',
                        headers: {'Content-Type': 'application/json'},
                        body: JSON.stringify({product_id: productId}),
                        credentials: 'same-origin'
                    });

                    const contentType = response.headers.get("content-type");
                    let data;
                    if (contentType && contentType.includes("application/json")) {
                        data = await response.json();
                    } else {
                        const textError = await response.text();
                        console.error("❌ El servidor no devolvió JSON. Respuesta:", textError);
                        alert(`Error del servidor (${response.status}). Revisa la consola.`);
                        return;
                    }

                    if (response.ok) {
                        console.log("✅ Éxito:", data.message);
                        // Despachamos el evento simple hacia el layout global
                        window.dispatchEvent(new CustomEvent('update-cart-count'));
                    } else {
                        console.error("❌ Error lógico de la API:", data);
                    }
                } catch (error) {
                    console.error("❌ Error de red/conexión:", error);
                    alert("Error de conexión. Verifica que el backend esté corriendo.");
                } finally {
                    this.addingToCart = null;
                }
            }
        },
        mounted() {
            console.log("✅ Shop App montada correctamente con", this.products.length, "productos.");
        }
    }).mount('#shop-app');
});