document.addEventListener('DOMContentLoaded', () => {
    const {createApp} = Vue;

    createApp({
        delimiters: ['[[', ']]'],
        data() {
            const el = document.getElementById('shop-app');
            const rawData = el ? el.getAttribute('data-products') : '[]';

            return {
                products: JSON.parse(rawData),
                searchQuery: ''
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
            }
        },
        mounted() {
            console.log("✅ Shop App montada correctamente con", this.products.length, "productos.");
        }
    }).mount('#shop-app');
});