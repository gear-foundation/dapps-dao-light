use dao_light::Vote;
use gtest::{Program, System};
pub mod utils;
use utils::*;

#[test]
fn funding_proposals() {
    let system = System::new();
    system.init_logger();
    let ftoken = Program::ftoken(&system);
    let dao = Program::dao(&system);
    let applicant: u64 = 200;
    let amount: u128 = 10_000;
    let share: u128 = 10_000;
    let quorum: u128 = 50;
    let mut proposal_id: u128 = 0;

    // add members to DAO
    for applicant in APPLICANTS {
        ftoken.mint(0, *applicant, *applicant, amount);
        ftoken.approve(1, *applicant, DAO_ID, amount);
        dao.deposit(*applicant, amount, share, false);
    }

    //funding proposal
    dao.submit_funding_proposal(APPLICANTS[0], proposal_id, applicant, amount, quorum, false);

    // members of DAO vote
    for applicant in APPLICANTS {
        let vote: Vote = if applicant < &16 { Vote::Yes } else { Vote::No };
        dao.submit_vote(*applicant, proposal_id, vote, false);
    }

    system.spend_blocks((VOTING_PERIOD_LENGTH as u32 + GRACE_PERIOD_LENGTH as u32 + 1000) / 1000);
    // proposal passed
    dao.process_proposal(proposal_id, true, false);

    // check balance of receiver
    ftoken.check_balance(applicant, amount);
    // check balance of DAO
    ftoken.check_balance(DAO_ID, 9 * amount);

    // new proposal
    proposal_id += 1;
    dao.submit_funding_proposal(APPLICANTS[0], proposal_id, applicant, amount, quorum, false);

    // DAO members vote
    for applicant in APPLICANTS {
        let vote: Vote = if applicant < &16 { Vote::No } else { Vote::Yes };
        dao.submit_vote(*applicant, proposal_id, vote, false);
    }

    system.spend_blocks((VOTING_PERIOD_LENGTH as u32 + GRACE_PERIOD_LENGTH as u32 + 1000) / 1000);

    // proposal didn't pass
    dao.process_proposal(proposal_id, false, false);

    // check balance of applicant
    ftoken.check_balance(applicant, amount);
    // check balance of DAO
    ftoken.check_balance(DAO_ID, 9 * amount);
}
