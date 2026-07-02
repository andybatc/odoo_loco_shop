document.addEventListener('DOMContentLoaded', () => {
    const { createApp } = Vue;

    createApp({
        delimiters: ['[[', ']]'],
        data() {
            const rawTotal = document.getElementById('cart-app')?.getAttribute('data-total') || '0';
            let items = [];
            const scriptEl = document.getElementById('cart-data');
            if (scriptEl) {
                try {
                    items = JSON.parse(scriptEl.textContent);
                } catch (e) {
                    console.error('Error parsing cart data:', e);
                    items = [];
                }
                if (!Array.isArray(items)) items = [];
            }
            return {
                items,
                totalBackend: parseFloat(rawTotal)
            }
        },
        computed: {
            // Recalcular dinámicamente en el cliente por seguridad reactiva
            totalGeneral() {
                return this.items.reduce((acc, item) => {
                    return acc + (item.price * item.quantity);
                }, 0);
            }
        },
        methods: {
            handleImageError(event) {
                event.target.src = '/static/images/No image available.jpeg';
            },
            async updateQuantity(item, newQty) {
                if (newQty < 1) return;
                try {
                    const res = await fetch(`/api/carts/items/${item.item_id}`, {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ quantity: newQty })
                    });
                    if (!res.ok) throw new Error('Error al actualizar');
                    item.quantity = newQty;
                    window.dispatchEvent(new CustomEvent('update-cart-count'));
                } catch(e) {
                    console.error('Error updating quantity:', e);
                    showToast('Error al actualizar cantidad');
                }
            },
            async removeItem(item) {
                if (!confirm('¿Eliminar este producto del carrito?')) return;
                try {
                    const res = await fetch(`/api/carts/items/${item.item_id}`, {
                        method: 'DELETE'
                    });
                    if (!res.ok) throw new Error('Error al eliminar');
                    const idx = this.items.indexOf(item);
                    if (idx > -1) this.items.splice(idx, 1);
                    window.dispatchEvent(new CustomEvent('update-cart-count'));
                } catch(e) {
                    console.error('Error removing item:', e);
                    showToast('Error al eliminar producto');
                }
            }
        },
        mounted() {
            console.log("🛒 App del Carrito montada con éxito. Ítems cargados:", this.items.length);
        }
    }).mount('#cart-app');
});