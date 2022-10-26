#![no_std]

use codec::{Decode, Encode};
use gstd::{ActorId, String};
use scale_info::TypeInfo;

#[derive(Debug, Decode, Encode, TypeInfo, Clone)]
pub enum DaoAction {
    /// Deposits tokens to DAO
    /// The account gets a share in DAO that is calculated as: (amount * self.total_shares / self.balance)
    ///
    /// On success replies with [`DaoEvent::Deposit`]
    Deposit {
        /// the number of fungible tokens that user wants to deposit to DAO
        amount: u128,
    },

    /// The proposal of funding.
    ///
    /// Requirements:
    ///
    /// * The proposal can be submitted only by the existing members;
    /// * The receiver ID can't be the zero;
    /// * The DAO must have enough funds to finance the proposal
    ///
    /// On success replies with [`DaoEvent::SubmitFundingProposal`]
    SubmitFundingProposal {
        /// an actor that will be funded
        receiver: ActorId,
        /// the number of fungible tokens that will be sent to the receiver
        amount: u128,
        /// a certain threshold of YES votes in order for the proposal to pass
        quorum: u128,
        /// the proposal description
        details: String,
    },

    /// The proposal processing after the proposal completes during the grace period.
    /// If the proposal is accepted, the indicated amount of tokens are sent to the receiver.
    ///
    /// Requirements:
    /// * The previous proposal must be processed;
    /// * The proposal must exist and be ready for processing;
    /// * The proposal must not be already be processed.
    ///
    /// On success replies with [`DaoEvent::ProcessProposal`]
    ProcessProposal {
        /// the proposal ID
        proposal_id: u128,
    },

    /// The member submit his vote (YES or NO) on the proposal.
    ///
    /// Requirements:
    /// * The proposal can be submitted only by the existing members;
    /// * The member can vote on the proposal only once;
    /// * Proposal must exist, the voting period must has started and not expired;
    ///
    ///  On success replies with [`DaoEvent::SubmitVote`]
    SubmitVote {
        /// the proposal ID
        proposal_id: u128,
        /// the member  a member vote (YES or NO)
        vote: Vote,
    },

    /// Withdraws the capital of the member
    ///
    /// Requirements:
    /// * `msg::source()` must be DAO member;
    /// * The member must have sufficient amount of shares;
    /// * The latest proposal the member voted YES must be processed;
    ///
    ///  On success replies with [`DaoEvent::RageQuit`]
    RageQuit {
        /// The amount of shares the member would like to withdraw
        amount: u128,
    },

    /// Continues the transaction if it fails due to lack of gas
    /// or due to an error in the token contract.
    ///
    /// Requirements:
    /// * Transaction must exist.
    ///
    /// On success replies with the DaoEvent of continued transaction.
    Continue(
        /// the transaction ID
        u64,
    ),
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum DaoEvent {
    Deposit {
        member: ActorId,
        share: u128,
    },
    SubmitFundingProposal {
        proposer: ActorId,
        receiver: ActorId,
        proposal_id: u128,
        amount: u128,
    },
    SubmitVote {
        account: ActorId,
        proposal_id: u128,
        vote: Vote,
    },
    ProcessProposal {
        proposal_id: u128,
        passed: bool,
    },
    RageQuit {
        member: ActorId,
        amount: u128,
    },
    TransactionFailed(u64),
}

#[derive(Debug, Decode, Encode, TypeInfo)]
pub struct InitDao {
    pub approved_token_program_id: ActorId,
    pub voting_period_length: u64,
    pub period_duration: u64,
    pub grace_period_length: u64,
}

#[derive(Debug, Encode, Decode, Clone, TypeInfo)]
pub enum Vote {
    Yes,
    No,
}
