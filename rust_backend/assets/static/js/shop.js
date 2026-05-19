const { createApp } = Vue;

createApp({
    delimiters: ['[[', ']]'],
    data() {
        return {
            // Leemos la variable global inyectada por el HTML
            products: window.__INITIAL_PRODUCTS__ || []
        }
    },
    mounted() {
        console.log("Productos cargados en Vue:", this.products);
    }
}).mount('#shop-app');