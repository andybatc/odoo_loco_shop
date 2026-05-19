// Aseguramos que el HTML esté listo antes de ejecutar Vue
document.addEventListener('DOMContentLoaded', () => {

    const { createApp } = Vue;

    createApp({
        delimiters: ['[[', ']]'],
        data() {
            return {
                contador: 0,
                menuAbierto: false
            }
        },
        methods: {
            incrementar() {
                this.contador++;
            }
        },
        mounted() {
            // Si ves esto al recargar la página, Vue arrancó bien
            console.log("✅ Vue montado correctamente en #vue-test");

            // Escuchamos el evento
            window.addEventListener('abrir-menu-rust', () => {
                this.menuAbierto = true;
                console.log("🎯 2. Evento recibido en Vue: Abriendo panel lateral");
            });
        }
    }).mount('#vue-test');
});