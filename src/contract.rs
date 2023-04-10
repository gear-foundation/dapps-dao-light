use crate::ft_messages::*;
use dao_light_io::*;
use gstd::{exec, msg, prelude::*, ActorId, String};
use hashbrown::HashMap;

#[derive(Debug, Default)]
pub struct Dao {
    pub approved_token_program_id: ActorId,
    pub period_duration: u64,
    pub voting_period_length: u64,
    pub grace_period_length: u64,
    pub total_shares: u128,
    pub members: HashMap<ActorId, Member>,
    pub proposal_id: u128,
    pub locked_funds: u128,
    pub proposals: HashMap<u128, Proposal>,
    pub balance: u128,
    pub transaction_id: u64,
    pub transactions: HashMap<u64, Option<DaoAction>>,
}

impl From<&Dao> for DaoState {
    fn from(dao: &Dao) -> Self {
        let members = dao.members.clone().into_iter().collect();
        let proposals = dao.proposals.clone().into_iter().collect();
        let transactions = dao.transactions.clone().into_iter().collect();

        DaoState {
            approved_token_program_id: dao.approved_token_program_id,
            period_duration: dao.period_duration,
            voting_period_length: dao.voting_period_length,
            grace_period_length: dao.grace_period_length,
            total_shares: dao.total_shares,
            members,
            proposal_id: dao.proposal_id,
            locked_funds: dao.locked_funds,
            proposals,
            balance: dao.balance,
            transaction_id: dao.transaction_id,
            transactions,
        }
    }
}

static mut DAO: Option<Dao> = None;

impl Dao {
    async fn deposit(&mut self, transaction_id: Option<u64>, amount: u128) {
        let current_transaction_id = self.get_transaction_id(transaction_id);
        if transfer_tokens(
            current_transaction_id,
            &self.approved_token_program_id,
            &msg::source(),
            &exec::program_id(),
            amount,
        )
        .await
        .is_err()
        {
            self.transactions.remove(&current_transaction_id);
            msg::reply(DaoEvent::TransactionFailed(current_transaction_id), 0)
                .expect("Error in a reply `DaoEvent::TransactionFailed`");
            return;
        };
        let share = self.calculate_share(amount);
        self.members
            .entry(msg::source())
            .and_modify(|member| member.shares = member.shares.saturating_add(share))
            .or_insert(Member {
                shares: share,
                highest_index_yes_vote: None,
            });

        self.total_shares = self.total_shares.saturating_add(share);
        self.balance = self.balance.saturating_add(amount);
        self.transactions.remove(&current_transaction_id);
        msg::reply(
            DaoEvent::Deposit {
                member: msg::source(),
                share,
            },
            0,
        )
        .expect("Error in a reply `DaoEvent::Deposit`");
    }

    fn submit_funding_proposal(
        &mut self,
        receiver: &ActorId,
        amount: u128,
        quorum: u128,
        details: String,
    ) {
        self.check_for_membership(&msg::source());

        if receiver.is_zero() {
            panic!("Proposal for the zero address");
        }

        // check that DAO has sufficient funds
        if self.balance.saturating_sub(self.locked_funds) < amount {
            panic!("Not enough funds in DAO");
        }

        let mut starting_period = exec::block_timestamp();
        let proposal_id = self.proposal_id;
        // compute startingPeriod for proposal
        // there should be a minimum time interval between proposals (period_duration) so that members have time to ragequit
        if proposal_id > 0 {
            let previous_starting_period = self
                .proposals
                .get(&(&proposal_id - 1))
                .expect("Cant be None")
                .starting_period;
            if starting_period < previous_starting_period + self.period_duration {
                starting_period = previous_starting_period + self.period_duration;
            }
        }

        let proposal = Proposal {
            proposer: msg::source(),
            receiver: *receiver,
            quorum,
            amount,
            details,
            starting_period,
            ended_at: starting_period + self.voting_period_length,
            ..Proposal::default()
        };

        self.proposals.insert(proposal_id, proposal);
        self.proposal_id = self.proposal_id.saturating_add(1);
        self.locked_funds = self.locked_funds.saturating_add(amount);

        msg::reply(
            DaoEvent::SubmitFundingProposal {
                proposer: msg::source(),
                receiver: *receiver,
                proposal_id,
                amount,
            },
            0,
        )
        .expect("Error in a reply `DaoEvent::SubmitFundingProposal`");
    }

    fn submit_vote(&mut self, proposal_id: u128, vote: Vote) {
        let member = self
            .members
            .get_mut(&msg::source())
            .expect("Account is not a member");

        // checks that proposal exists, the voting period has started, not expired and that member did not vote on the proposal
        let proposal = match self.proposals.get_mut(&proposal_id) {
            Some(proposal) => {
                if exec::block_timestamp() > proposal.starting_period + self.voting_period_length {
                    panic!("proposal voting period has expired");
                }
                if exec::block_timestamp() < proposal.starting_period {
                    panic!("voting period has not started");
                }
                if proposal
                    .votes_by_member
                    .iter()
                    .any(|(actor, _vote)| msg::source().eq(actor))
                {
                    panic!("account has already voted on that proposal");
                }
                proposal
            }
            None => {
                panic!("proposal does not exist");
            }
        };

        match vote {
            Vote::Yes => {
                proposal.yes_votes = proposal.yes_votes.saturating_add(member.shares);
                // it is necessary to save the highest id of the proposal - must be processed for member to ragequit
                if let Some(id) = member.highest_index_yes_vote {
                    if id < proposal_id {
                        member.highest_index_yes_vote = Some(proposal_id);
                    }
                } else {
                    member.highest_index_yes_vote = Some(proposal_id);
                }
            }
            Vote::No => {
                proposal.no_votes = proposal.no_votes.saturating_add(member.shares);
            }
        }
        proposal.votes_by_member.push((msg::source(), vote.clone()));

        msg::reply(
            DaoEvent::SubmitVote {
                account: msg::source(),
                proposal_id,
                vote,
            },
            0,
        )
        .expect("Error in a reply `DaoEvent::SubmitVote`");
    }

    async fn process_proposal(&mut self, transaction_id: Option<u64>, proposal_id: u128) {
        let current_transaction_id = self.get_transaction_id(transaction_id);
        if proposal_id > 0
            && !self
                .proposals
                .get(&(&proposal_id - 1))
                .expect("Cant be None")
                .processed
        {
            panic!("Previous proposal must be processed");
        }
        let proposal = match self.proposals.get_mut(&proposal_id) {
            Some(proposal) => {
                if proposal.processed {
                    panic!("Proposal has already been processed");
                }
                if exec::block_timestamp()
                    < proposal.starting_period
                        + self.voting_period_length
                        + self.grace_period_length
                {
                    panic!("Proposal is not ready to be processed");
                }
                proposal
            }
            None => {
                panic!("proposal does not exist");
            }
        };

        proposal.passed = proposal.yes_votes > proposal.no_votes
            && proposal.yes_votes * 10_000 / self.total_shares >= proposal.quorum * 100;

        // if funding propoposal has passed
        if proposal.passed
            && transfer_tokens(
                current_transaction_id,
                &self.approved_token_program_id,
                &exec::program_id(),
                &proposal.receiver,
                proposal.amount,
            )
            .await
            .is_err()
        {
            msg::reply(DaoEvent::TransactionFailed(current_transaction_id), 0)
                .expect("Error in a reply `DaoEvent::TransactionFailed`");
            return;
        };

        self.locked_funds = self.locked_funds.saturating_sub(proposal.amount);
        self.balance = self.balance.saturating_sub(proposal.amount);
        self.transactions.remove(&current_transaction_id);
        proposal.processed = true;
        msg::reply(
            DaoEvent::ProcessProposal {
                proposal_id,
                passed: proposal.passed,
            },
            0,
        )
        .expect("Error in a reply `DaoEvent::ProcessProposal`");
    }

    async fn ragequit(&mut self, transaction_id: Option<u64>, amount: u128) {
        let current_transaction_id = self.get_transaction_id(transaction_id);
        let funds = self.redeemable_funds(amount);
        let member = self
            .members
            .get_mut(&msg::source())
            .expect("Account is not a DAO member");
        if amount > member.shares {
            panic!("unsufficient shares");
        }
        if let Some(proposal_id) = member.highest_index_yes_vote {
            if let Some(proposal) = self.proposals.get(&proposal_id) {
                if !proposal.processed {
                    panic!("cant ragequit until highest index proposal member voted YES on is processed");
                }
            }
        }

        if transfer_tokens(
            current_transaction_id,
            &self.approved_token_program_id,
            &exec::program_id(),
            &msg::source(),
            funds,
        )
        .await
        .is_err()
        {
            msg::reply(DaoEvent::TransactionFailed(current_transaction_id), 0)
                .expect("Error in a reply `DaoEvent::TransactionFailed`");
            return;
        };
        member.shares = member.shares.saturating_sub(amount);
        self.transactions.remove(&current_transaction_id);
        self.total_shares = self.total_shares.saturating_sub(amount);
        self.balance = self.balance.saturating_sub(funds);
        msg::reply(
            DaoEvent::RageQuit {
                member: msg::source(),
                amount: funds,
            },
            0,
        )
        .expect("Error in a reply `DaoEvent::RageQuit`");
    }

    async fn continue_transaction(&mut self, transaction_id: u64) {
        let transactions = self.transactions.clone();
        let payload = &transactions
            .get(&transaction_id)
            .expect("Transaction does not exist");
        if let Some(action) = payload {
            match action {
                DaoAction::Deposit { amount } => {
                    self.deposit(Some(transaction_id), *amount).await;
                }
                DaoAction::ProcessProposal { proposal_id } => {
                    self.process_proposal(Some(transaction_id), *proposal_id)
                        .await;
                }
                DaoAction::RageQuit { amount } => {
                    self.ragequit(Some(transaction_id), *amount).await;
                }
                _ => unreachable!(),
            }
        }
    }
}

#[no_mangle]
extern "C" fn init() {
    let config: InitDao = msg::load().expect("Unable to decode InitDao");
    let dao = Dao {
        approved_token_program_id: config.approved_token_program_id,
        voting_period_length: config.voting_period_length,
        period_duration: config.period_duration,
        ..Dao::default()
    };
    unsafe { DAO = Some(dao) };
}

#[gstd::async_main]
async fn main() {
    let action: DaoAction = msg::load().expect("Could not load Action");
    let dao: &mut Dao = unsafe { DAO.get_or_insert(Dao::default()) };
    match action {
        DaoAction::Deposit { amount } => {
            dao.transactions.insert(dao.transaction_id, Some(action));
            dao.deposit(None, amount).await;
        }
        DaoAction::SubmitFundingProposal {
            receiver,
            amount,
            quorum,
            details,
        } => {
            dao.submit_funding_proposal(&receiver, amount, quorum, details);
        }
        DaoAction::ProcessProposal { proposal_id } => {
            dao.transactions.insert(dao.transaction_id, Some(action));
            dao.process_proposal(None, proposal_id).await;
        }
        DaoAction::SubmitVote { proposal_id, vote } => {
            dao.submit_vote(proposal_id, vote);
        }
        DaoAction::RageQuit { amount } => {
            dao.transactions.insert(dao.transaction_id, Some(action));
            dao.ragequit(None, amount).await;
        }
        DaoAction::Continue(transaction_id) => dao.continue_transaction(transaction_id).await,
    }
}

#[no_mangle]
extern "C" fn state() {
    msg::reply(
        unsafe {
            let dao = DAO.as_ref().expect("Uninitialized dao state");
            let dao_state: DaoState = dao.into();
            dao_state
        },
        0,
    )
    .expect("Failed to encode or reply with `<AppMetadata as Metadata>::State` from `state()`");
}

#[no_mangle]
extern "C" fn metahash() {
    msg::reply::<[u8; 32]>(include!("../.metahash"), 0)
        .expect("Failed to encode or reply with `[u8; 32]` from `metahash()`");
}
