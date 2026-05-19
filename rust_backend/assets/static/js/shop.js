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
    mounted() {
        console.log("Productos montados correctamente:", this.products);
    }
}).mount('#shop-app');