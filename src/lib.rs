use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, near_bindgen, AccountId, Balance, BorshStorageKey, PanicOnDefault, Promise, PromiseOrValue,
};

mod order;
use order::*;

pub type OrderId = String;

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
#[near_bindgen]
struct EcommerceContract {
    pub owner_id: AccountId,
    pub orders: LookupMap<OrderId, Order>,
}

#[derive(BorshDeserialize, BorshSerialize, BorshStorageKey)]
enum StorageKey {
    OrderKey,
}

/**
 * Storage account key/ value,  ["Vu Nguyen", "Vu nguyen 123", "Vu nguyen abc"] = names
 * {key: value, number: 1, name: "Vu Nguyen"}
 * {"names[0]": "Vu nguyen", "names[1]": "Vu nguyen 123", key: value, number: 1}
 */
/**
 * Bài tập về nhà: Cho phép owner trả tiền lại cho user khi user muốn trả hàng
 */

#[near_bindgen]
impl EcommerceContract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        Self {
            owner_id,
            orders: LookupMap::new(StorageKey::OrderKey),
        }
    }

    #[payable]
    pub fn pay_order(&mut self, order_id: OrderId, order_amount: U128) -> PromiseOrValue<U128> {
        // Lay thong tin so NEAR deposit cua user env::attached_deposit()
        assert!(
            env::attached_deposit() >= order_amount.0,
            "ERROR_DEPOSIT_NOT_ENOUGH"
        );

        // Luu tru lai thong tin thanh toan cua user
        let order: Order = Order {
            order_id: order_id.clone(),
            payer_id: env::signer_account_id(),
            amount: order_amount.0,
            received_amount: env::attached_deposit(),
            is_completed: true,
            is_refund: false,
            created_at: env::block_timestamp(),
        };

        self.orders.insert(&order_id, &order);

        // Tra lai tien thua cho user
        if env::attached_deposit() > order_amount.0 {
            Promise::new(env::signer_account_id())
                .transfer(env::attached_deposit() - order_amount.0);
            PromiseOrValue::Value(U128(env::attached_deposit() - order_amount.0))
        } else {
            PromiseOrValue::Value(U128(0))
        }
    }

    pub fn get_order(&self, order_id: OrderId) -> Order {
        self.orders.get(&order_id).expect("NOT_FOUND_ORDER_ID")
    }

    #[payable]
    fn refund_order(&mut self, order_id: String) -> PromiseOrValue<Order> {
        let mut order = self.get_order(order_id);

        // Lay thong tin so NEAR deposit cua minh env::account_balance()
        println!("Balance: {}", env::account_balance().to_string());
        println!("Order amount: {}", order.amount.to_string());
        println!(
            "Order amount: {}",
            (env::account_balance() - order.amount).to_string()
        );
        assert!(
            env::account_balance() > order.amount,
            "ERROR_DEPOSIT_NOT_ENOUGH"
        );

        println!("Pass assert");
        if env::account_balance() >= order.amount {
            order.refund();
            println!("Order amount: {}", order.amount.to_string());
            self.orders.insert(&order.order_id, &order);
            Promise::new(env::current_account_id()).transfer(order.amount);
            PromiseOrValue::Value(order)
        } else {
            PromiseOrValue::Value(order)
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, MockedBlockchain};

    fn get_context(is_view: bool) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(accounts(0))
            .predecessor_account_id(accounts(0))
            .is_view(is_view);

        builder
    }

    #[test]
    fn test_pay_order() {
        let mut context = get_context(false);
        let alice: AccountId = accounts(0);

        context
            .account_balance(1000)
            .predecessor_account_id(alice.clone())
            .attached_deposit(1000)
            .signer_account_id(alice.clone());

        testing_env!(context.build());

        let mut contract = EcommerceContract::new(alice.clone());
        let order_amount = U128(1000);
        contract.pay_order("order_1".to_owned(), order_amount);

        let order = contract.get_order("order_1".to_owned());

        // Test
        assert_eq!(order.order_id, "order_1".to_owned());
        assert_eq!(order.amount, order_amount.0);
        assert_eq!(order.payer_id, alice);
        assert!(order.is_completed);
    }

    #[test]
    #[should_panic(expected = "ERROR_DEPOSIT_NOT_ENOUGH")]
    fn test_pay_order_with_lack_balance() {
        let mut context = get_context(false);
        let alice: AccountId = accounts(0);

        context
            .account_balance(1000)
            .predecessor_account_id(alice.clone())
            .attached_deposit(1000)
            .signer_account_id(alice.clone());

        testing_env!(context.build());

        let mut contract = EcommerceContract::new(alice.clone());
        let order_amount = U128(2000);
        contract.pay_order("order_1".to_owned(), order_amount);
    }

    #[test]
    fn test_refund_order() {
        let mut context = get_context(false);
        let alice: AccountId = accounts(0);

        context
            .account_balance(2000)
            .predecessor_account_id(alice.clone())
            .attached_deposit(1000)
            .signer_account_id(alice.clone());

        testing_env!(context.build());

        let mut contract = EcommerceContract::new(alice.clone());
        // contract.pay_order("order_1".to_owned(), order_amount);
        let order_id = "order_1".to_owned();
        let order_amount = U128(1000);
        let order: Order = Order {
            order_id: order_id.clone(),
            payer_id: alice.clone(),
            amount: order_amount.0,
            received_amount: order_amount.0,
            is_completed: true,
            is_refund: false,
            created_at: env::block_timestamp(),
        };
        contract.orders.insert(&order_id, &order);

        match contract.refund_order("order_1".to_owned()) {
            PromiseOrValue::Value(refunded_order) => {
                // Can refund
                assert_eq!(refunded_order.order_id, "order_1".to_owned());
                assert_eq!(refunded_order.amount, order_amount.0);
                assert_eq!(refunded_order.payer_id, alice);
                assert_eq!(refunded_order.is_refund, true);
                assert!(refunded_order.is_completed);
            }
            _ => {}
        }
    }

    #[test]
    #[should_panic(expected = "ERROR_DEPOSIT_NOT_ENOUGH")]
    fn test_refund_order_with_lack_balance() {
        let mut context = get_context(false);
        let alice: AccountId = accounts(0);

        context
            .account_balance(0)
            .predecessor_account_id(alice.clone())
            .attached_deposit(1000)
            .signer_account_id(alice.clone());

        testing_env!(context.build());

        let mut contract = EcommerceContract::new(alice.clone());
        let order_id = "order_1".to_owned();
        let order_amount = U128(1000);
        let order: Order = Order {
            order_id: order_id.clone(),
            payer_id: alice.clone(),
            amount: order_amount.0,
            received_amount: order_amount.0,
            is_completed: true,
            is_refund: false,
            created_at: env::block_timestamp(),
        };
        contract.orders.insert(&order_id, &order);

        contract.refund_order("order_1".to_owned());
    }
}
