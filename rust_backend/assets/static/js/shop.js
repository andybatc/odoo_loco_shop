const { createApp } = Vue;

createApp({
    delimiters: ['[[', ']]'],
    data() {
        // Obtenemos el elemento donde montamos la app
        const el = document.getElementById('shop-app');

        // Leemos el JSON del atributo 'data-products'
        const rawData = el ? el.getAttribute('data-products') : '[]';

        return {
            products: JSON.parse(rawData)
        }
    },
    methods: {
        handleImageError(event) {
            event.target.src = '/static/images/No image avaible.jpeg';
        }
    },
    mounted() {
        console.log("Productos montados correctamente:", this.products);
    }
}).mount('#shop-app');