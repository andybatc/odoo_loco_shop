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
            },
            updateCartBadge() {
                const el = document.getElementById('cart-count');
                if (el) {
                    el.textContent = this.cartCount;
                    const badge = el.closest('#cart-badge');
                    if (badge) {
                        if (this.cartCount > 0) {
                            badge.classList.remove('scale-0');
                        } else {
                            badge.classList.add('scale-0');
                        }
                    }
                }
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
                    this.cartCount++;
                }
                this.updateCartBadge();
                console.log("🛒 Carrito actualizado. Nuevo total:", this.cartCount);
            });

            document.getElementById('search-toggle')?.addEventListener('click', () => {
                const bar = document.getElementById('search-bar');
                if (bar) {
                    bar.classList.toggle('hidden');
                    bar.classList.toggle('block');
                    const input = bar.querySelector('input');
                    if (!bar.classList.contains('hidden')) input?.focus();
                }
            });
        }
    }).mount('#vue-test');
});