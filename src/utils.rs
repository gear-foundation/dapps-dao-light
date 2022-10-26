use crate::*;

impl Dao {
    // calculates the funds that the member can redeem based on his shares
    pub fn redeemable_funds(&self, share: u128) -> u128 {
        if self.total_shares > 0 {
            (share.saturating_mul(self.balance)) / self.total_shares
        } else {
            panic!("Zero total shares in DAO!");
        }
    }

    // calculates a share a user can receive for his deposited tokens
    pub fn calculate_share(&self, tokens: u128) -> u128 {
        if self.balance == 0 {
            return tokens;
        }
        (self.total_shares * tokens) / self.balance
    }

    pub fn is_member(&self, account: &ActorId) -> bool {
        matches!(self.members.get(account), Some(member) if member.shares != 0)
    }
    // checks that account is DAO member
    pub fn check_for_membership(&self, account: &ActorId) {
        if !self.is_member(account) {
            panic!("account is not a DAO member");
        }
    }

    // Determine either this is a new transaction
    // or the transaction which has to be completed
    pub fn get_transaction_id(&mut self, transaction_id: Option<u64>) -> u64 {
        match transaction_id {
            Some(transaction_id) => transaction_id,
            None => {
                let transaction_id = self.transaction_id;
                self.transaction_id = self.transaction_id.wrapping_add(1);
                transaction_id
            }
        }
    }
}
