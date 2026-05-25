// Aseguramos que el HTML esté listo antes de ejecutar Vue
document.addEventListener('DOMContentLoaded', () => {

    const { createApp } = Vue;

    createApp({
        delimiters: ['[[', ']]'],
        data() {
            return {
                contador: 0,
                menuAbierto: false,
                cartCount: 0
            }
        },
        methods: {
            incrementar() {
                this.contador++;
            }
        },
        mounted() {
            console.log("✅ Vue montado correctamente en #vue-test");

            window.addEventListener('abrir-menu-rust', () => {
                this.menuAbierto = true;
                console.log("🎯 2. Evento recibido en Vue: Abriendo panel lateral");
            });

            // Escuchamos el aviso de la tienda de forma segura
            window.addEventListener('update-cart-count', (event) => {
                if (event && event.detail && typeof event.detail.count !== 'undefined') {
                    this.cartCount = event.detail.count;
                } else {
                    this.cartCount++; // Suma 1 limpiamente sobre el 0 inicial
                }
                console.log("🛒 Carrito actualizado en layout global. Nuevo total:", this.cartCount);
            });
        }
    }).mount('#vue-test');
});