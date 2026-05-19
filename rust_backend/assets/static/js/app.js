// Asegúrate de que vue.global.js se carga ANTES que este archivo
const { createApp } = Vue;

createApp({
    // Definimos delimitadores custom para que no choquen con Tera (Rust)
    delimiters: ['[[', ']]'],

    // Estado inicial
    data() {
        return {
            contador: 0
        }
    },

    // Acciones y métodos reactivos
    methods: {
        incrementar() {
            // Usamos 'this' para acceder a la variable del estado
            this.contador++;
            console.log("Contador incrementado a:", this.contador);
        }
    }
}).mount('#vue-test');