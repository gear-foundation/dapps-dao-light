use dao_light_io::*;
use gtest::{Program, System};
pub mod utils;
use utils::*;

#[test]
fn submit_funding_proposal() {
    let system = System::new();
    system.init_logger();
    let ftoken = Program::ftoken(&system);
    let dao = Program::dao(&system);
    let applicant: u64 = 200;
    let amount: u128 = 10_000;
    let share: u128 = 10_000;
    let quorum: u128 = 50;
    let proposal_id: u128 = 0;

    let user = 1000;
    // must fail since account is not a member
    dao.submit_funding_proposal(user, proposal_id, applicant, amount, quorum, true);

    // add user to DAO
    ftoken.mint(0, user, user, amount);
    ftoken.approve(1, user, DAO_ID, amount);

    dao.deposit(user, amount, share, false);

    // must fail since not enough funds in DAO
    dao.submit_funding_proposal(user, proposal_id, applicant, amount + 1, quorum, true);

    // must fail since proposal for a zero address
    dao.submit_funding_proposal(user, proposal_id, 0, amount, quorum, true);
}

#[test]
fn submit_vote() {
    let system = System::new();
    system.init_logger();
    let ftoken = Program::ftoken(&system);
    let dao = Program::dao(&system);
    let applicant: u64 = 200;
    let amount: u128 = 10_000;
    let share: u128 = 10_000;
    let quorum: u128 = 50;
    let proposal_id: u128 = 0;

    let user = 1000;

    // add user to DAO
    ftoken.mint(0, user, user, amount);
    ftoken.approve(1, user, DAO_ID, amount);

    dao.deposit(user, amount, share, false);

    // must fail since not enough funds in DAO
    dao.submit_funding_proposal(user, proposal_id, applicant, amount / 2, quorum, false);

    // must fail since the the account is not a member nor a delegate
    dao.submit_vote(user + 1, proposal_id, Vote::Yes, true);

    // submit another proposalo
    dao.submit_funding_proposal(user, proposal_id + 1, applicant, amount / 10, quorum, false);

    // must fail since the voting period has not started
    dao.submit_vote(user, proposal_id + 1, Vote::Yes, true);

    system.spend_blocks((PERIOD_DURATION / 1000) as u32);

    dao.submit_vote(user, proposal_id + 1, Vote::Yes, false);

    // must fail since the account has already voted on this proposal
    dao.submit_vote(user, proposal_id + 1, Vote::No, true);

    system.spend_blocks(((VOTING_PERIOD_LENGTH + 1000) / 1000) as u32);
    // must fail since the proposal voting period has expired
    dao.submit_vote(user, proposal_id, Vote::Yes, true);

    // must fail since the proposal does not exist
    dao.submit_vote(user, proposal_id + 2, Vote::Yes, true);
}

#[test]
fn process_proposal() {
    let system = System::new();
    system.init_logger();
    let ftoken = Program::ftoken(&system);
    let dao = Program::dao(&system);
    let applicant: u64 = 200;
    let amount: u128 = 10_000;
    let share: u128 = 10_000;
    let quorum: u128 = 50;
    let proposal_id: u128 = 0;

    let user = 1000;

    // add user to DAO
    ftoken.mint(0, user, user, amount);
    ftoken.approve(1, user, DAO_ID, amount);

    dao.deposit(user, amount, share, false);

    // must fail since proposal does not exist
    dao.process_proposal(proposal_id, true, true);

    // submit proposal
    dao.submit_funding_proposal(user, proposal_id, applicant, amount / 10, quorum, false);

    // submit another proposal
    dao.submit_funding_proposal(user, proposal_id + 1, applicant, amount / 10, quorum, false);

    // must fail since previous proposal must be processed
    dao.process_proposal(proposal_id + 1, true, true);

    // must fail since the proposal is not ready to be processed
    dao.process_proposal(proposal_id, true, true);

    system.spend_blocks(((VOTING_PERIOD_LENGTH + GRACE_PERIOD_LENGTH) / 1000) as u32);

    dao.process_proposal(proposal_id, false, false);

    // must fail since the proposal has already been processed
    dao.process_proposal(proposal_id, true, true);
}

#[test]
fn ragequit() {
    let system = System::new();
    system.init_logger();
    let ftoken = Program::ftoken(&system);
    let dao = Program::dao(&system);
    let applicant: u64 = 200;
    let amount: u128 = 10_000;
    let share: u128 = 10_000;
    let ragequit_amount: u128 = 6_000;
    let quorum: u128 = 50;
    let proposal_id: u128 = 0;

    // add members to DAO
    for applicant in APPLICANTS {
        ftoken.mint(0, *applicant, *applicant, amount);
        ftoken.approve(1, *applicant, DAO_ID, amount);
        dao.deposit(*applicant, amount, share, false);
    }

    // submit proposal
    dao.submit_funding_proposal(
        APPLICANTS[0],
        proposal_id,
        applicant,
        5 * amount,
        quorum,
        false,
    );

    // members of DAO vote
    for applicant in APPLICANTS {
        let vote: Vote = if applicant < &16 { Vote::Yes } else { Vote::No };
        dao.submit_vote(*applicant, proposal_id, vote, false);
    }

    // must fail since the applicant voted YES and the proposal has not been processed
    dao.ragequit(APPLICANTS[0], ragequit_amount, 0, true);

    // must fail since an account is not a DAO member
    dao.ragequit(300, ragequit_amount, 0, true);

    // must fail since a memeber has unsufficient shares
    dao.ragequit(APPLICANTS[8], 2 * ragequit_amount, 0, true);

    // successfull ragequit
    let mut balance: u128 = 10 * amount;
    let mut total_shares: u128 = 10 * amount;
    ftoken.check_balance(APPLICANTS[8], 0);
    let funds = (balance * ragequit_amount) / (total_shares);
    dao.ragequit(APPLICANTS[8], ragequit_amount, funds, false);
    total_shares -= ragequit_amount;
    balance -= funds;
    ftoken.check_balance(APPLICANTS[8], funds);
    ftoken.check_balance(DAO_ID, balance);

    // successfull ragequit
    ftoken.check_balance(APPLICANTS[9], 0);
    let funds = (balance * ragequit_amount) / (total_shares);
    dao.ragequit(APPLICANTS[9], ragequit_amount, funds, false);
    balance -= funds;
    total_shares -= ragequit_amount;
    ftoken.check_balance(APPLICANTS[9], funds);
    ftoken.check_balance(DAO_ID, balance);

    system.spend_blocks(((VOTING_PERIOD_LENGTH + GRACE_PERIOD_LENGTH) / 1000) as u32);

    dao.process_proposal(proposal_id, true, false);
    balance -= 5 * amount;
    ftoken.check_balance(DAO_ID, balance);
    ftoken.check_balance(applicant, 5 * amount);

    // successfull ragequit
    ftoken.check_balance(APPLICANTS[0], 0);
    let funds = (balance * ragequit_amount) / (total_shares);
    dao.ragequit(APPLICANTS[0], ragequit_amount, funds, false);
    balance -= funds;
    ftoken.check_balance(APPLICANTS[0], funds);
    ftoken.check_balance(DAO_ID, balance);
}
