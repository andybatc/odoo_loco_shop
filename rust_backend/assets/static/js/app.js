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

            // 🔥 Escuchamos cuando un producto es añadido al carrito desde el otro script
            window.addEventListener('update-cart-count', (event) => {
                // El servidor de Loco nos devuelve el estado actual o sumamos uno
                this.cartCount = event.detail.count;
                console.log("🛒 Carrito actualizado en layout global. Cantidad:", this.cartCount);
            });
        }
    }).mount('#vue-test');
});