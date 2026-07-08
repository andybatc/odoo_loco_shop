use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "orders")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Option<i32>,
    pub customer_name: String,
    pub customer_email: String,
    pub customer_phone: Option<String>,
    pub customer_street: Option<String>,
    pub customer_city: Option<String>,
    pub customer_zip: Option<String>,
    pub odoo_order_name: Option<String>,
    pub odoo_invoice_name: Option<String>,
    pub total: Decimal,
    pub status: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub shipping_cost: Option<Decimal>,
    pub customer_country: Option<String>,
    pub customer_state: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::order_items::Entity")]
    OrderItems,
}

impl Related<super::order_items::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrderItems.def()
    }
}
