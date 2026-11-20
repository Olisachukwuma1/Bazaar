/**
 * Contract error code → user-friendly message mapping.
 *
 * The marketplace contract (`contracts/ai-agent-marketplace/src/lib.rs`)
 * declares a `#[contracterror] enum Error` whose discriminants are surfaced by
 * the Soroban RPC as `HostError: Error(Contract, #n)`. This module parses that
 * format and turns it into a title + hint pair suitable for display.
 */

export interface ContractErrorInfo {
  /** Numeric contract error code, or `null` when it could not be parsed. */
  code: number | null;
  /** Variant name from the contract's `Error` enum, when known. */
  name: string | null;
  /** Short, user-facing title. */
  title: string;
  /** Actionable hint telling the user what to do next. */
  hint: string;
  /** `true` when the code is not part of the known mapping. */
  unknown: boolean;
}

interface ErrorEntry {
  name: string;
  title: string;
  hint: string;
}

/** Codes 1-51 of the contract `Error` enum. */
const CONTRACT_ERRORS: Record<number, ErrorEntry> = {
  1: { name: 'NotInitialized', title: 'Marketplace not initialized', hint: 'The contract has not been set up yet. Contact the marketplace operator.' },
  2: { name: 'AlreadyInitialized', title: 'Marketplace already initialized', hint: 'No action needed — the contract is already set up.' },
  3: { name: 'Unauthorized', title: 'Not authorized', hint: 'Connect the wallet that owns this record and try again.' },
  4: { name: 'AgentNotFound', title: 'Agent not found', hint: 'Register an agent profile before performing this action.' },
  5: { name: 'AgentAlreadyExists', title: 'Agent already registered', hint: 'This wallet already has an agent profile. Edit it from your dashboard.' },
  6: { name: 'AgentNotActive', title: 'Agent is inactive', hint: 'Reactivate your agent profile before bidding or accepting work.' },
  7: { name: 'JobNotFound', title: 'Job not found', hint: 'The job may have been removed. Refresh the marketplace list.' },
  8: { name: 'JobNotOpen', title: 'Job is no longer open', hint: 'This job has already been assigned or closed.' },
  9: { name: 'JobNotInProgress', title: 'Job is not in progress', hint: 'This action is only available while the job is in progress.' },
  10: { name: 'JobNotCompleted', title: 'Job not completed', hint: 'Wait for the agent to submit a deliverable first.' },
  11: { name: 'JobAlreadyAssigned', title: 'Job already assigned', hint: 'A bid has already been accepted for this job.' },
  12: { name: 'InsufficientBudget', title: 'Budget too low', hint: 'Increase the job budget to cover the bid and platform fee.' },
  13: { name: 'BidNotFound', title: 'Bid not found', hint: 'Refresh the job page — the bid may have been withdrawn.' },
  14: { name: 'BidNotPending', title: 'Bid is no longer pending', hint: 'This bid was already accepted, rejected or withdrawn.' },
  15: { name: 'BidAlreadyExists', title: 'Bid already placed', hint: 'You already have a bid on this job. Withdraw it before bidding again.' },
  16: { name: 'InsufficientBidAmount', title: 'Bid amount too low', hint: 'Enter a bid amount above zero.' },
  17: { name: 'BidTooLow', title: 'Bid below minimum', hint: 'Raise your bid to at least the job budget.' },
  18: { name: 'AgentReputationTooLow', title: 'Reputation too low', hint: 'Complete more jobs successfully to raise your reputation.' },
  19: { name: 'InsufficientSkills', title: 'Missing required skills', hint: 'Add the job’s required skills to your agent profile.' },
  20: { name: 'SkillNotVerified', title: 'Skill not verified', hint: 'Get the required skill verified by an authorized verifier.' },
  21: { name: 'EscrowNotFound', title: 'Escrow not found', hint: 'Escrow is created when a bid is accepted. Refresh the job page.' },
  22: { name: 'EscrowNotFunded', title: 'Escrow not funded', hint: 'Fund the escrow before releasing or disputing it.' },
  23: { name: 'EscrowAlreadyReleased', title: 'Escrow already released', hint: 'Funds for this job have already been paid out.' },
  24: { name: 'EscrowNotDisputed', title: 'Escrow not disputed', hint: 'Raise a dispute before trying to resolve it.' },
  25: { name: 'InsufficientEscrowBalance', title: 'Escrow balance too low', hint: 'The escrow does not hold enough funds for this payout.' },
  26: { name: 'InvalidDeadline', title: 'Invalid deadline', hint: 'Choose a deadline in the future.' },
  27: { name: 'InvalidBudget', title: 'Invalid budget', hint: 'Enter a positive budget amount.' },
  28: { name: 'InvalidRating', title: 'Invalid rating', hint: 'Ratings must be between 1 and 5.' },
  29: { name: 'ReviewNotFound', title: 'Review not found', hint: 'Refresh the page — the review may have been removed.' },
  30: { name: 'AlreadyReviewed', title: 'Already reviewed', hint: 'You have already reviewed this job.' },
  31: { name: 'DisputeNotFound', title: 'Dispute not found', hint: 'Refresh the job page to load the latest dispute state.' },
  32: { name: 'DisputeNotOpen', title: 'Dispute is not open', hint: 'This dispute has already been resolved.' },
  33: { name: 'DisputeWindowExpired', title: 'Dispute window expired', hint: 'The dispute period for this job has ended.' },
  34: { name: 'UnauthorizedResolver', title: 'Not an authorized resolver', hint: 'Only the marketplace admin can resolve disputes.' },
  35: { name: 'InvalidToken', title: 'Invalid payment token', hint: 'Use the USDC token address configured for this network.' },
  36: { name: 'InsufficientBalance', title: 'Insufficient USDC balance', hint: 'Add USDC to your wallet and try again.' },
  37: { name: 'TransferFailed', title: 'Token transfer failed', hint: 'Check your balance and token approval, then retry.' },
  38: { name: 'EscrowExpired', title: 'Escrow expired', hint: 'The escrow period has ended; the client may reclaim the funds.' },
  39: { name: 'JobExpired', title: 'Job expired', hint: 'This job passed its deadline and can no longer be acted on.' },
  40: { name: 'InvalidSkill', title: 'Invalid skill', hint: 'Pick skills from the supported list.' },
  41: { name: 'SkillAlreadyVerified', title: 'Skill already verified', hint: 'No action needed — this skill is already verified.' },
  42: { name: 'VerifierNotAuthorized', title: 'Not an authorized verifier', hint: 'Only authorized verifiers can verify skills.' },
  43: { name: 'VerificationExpired', title: 'Skill verification expired', hint: 'Re-verify the skill with an authorized verifier.' },
  44: { name: 'PlatformNotInitialized', title: 'Platform not initialized', hint: 'The platform config is missing. Contact the marketplace operator.' },
  45: { name: 'InvalidConfig', title: 'Invalid configuration', hint: 'The platform configuration is invalid. Contact the marketplace operator.' },
  46: { name: 'JobCancelled', title: 'Job cancelled', hint: 'This job was cancelled and can no longer be acted on.' },
  47: { name: 'AlreadyDisputed', title: 'Dispute already raised', hint: 'A dispute is already open for this job.' },
  48: { name: 'DisputeResolved', title: 'Dispute already resolved', hint: 'This dispute has been settled.' },
  49: { name: 'EscrowAlreadyRefunded', title: 'Escrow already refunded', hint: 'The escrow funds have already been returned to the client.' },
  50: { name: 'InvalidDisputeEvidence', title: 'Invalid dispute evidence', hint: 'Provide a non-empty evidence link or description.' },
  51: { name: 'ContractPaused', title: 'Marketplace paused', hint: 'The marketplace is temporarily paused. Try again later.' },
};

/** Matches `Error(Contract, #8)` and `Error(Contract, #8, ...)` SDK formats. */
const CONTRACT_ERROR_RE = /Error\(\s*Contract\s*,\s*#(\d+)/;

/** Extract the numeric contract error code from an SDK error string, if any. */
export function parseContractErrorCode(error: unknown): number | null {
  const text =
    typeof error === 'string'
      ? error
      : error instanceof Error
        ? error.message
        : error && typeof error === 'object' && 'message' in error
          ? String((error as { message: unknown }).message)
          : '';
  const match = CONTRACT_ERROR_RE.exec(text);
  return match ? Number(match[1]) : null;
}

/** Look up the friendly title/hint for a numeric contract error code. */
export function getContractErrorInfo(code: number | null): ContractErrorInfo {
  const entry = code === null ? undefined : CONTRACT_ERRORS[code];
  if (!entry) {
    return {
      code,
      name: null,
      title: code === null ? 'Transaction failed' : `Contract error #${code}`,
      hint:
        code === null
          ? 'The transaction could not be completed. Please try again.'
          : 'This error is not recognized by the app. Please try again or contact support.',
      unknown: true,
    };
  }
  return { code, name: entry.name, title: entry.title, hint: entry.hint, unknown: false };
}

/**
 * Turn any thrown value (SDK simulation error, submit error, plain `Error`)
 * into a user-friendly message. Unknown codes and non-contract errors fall
 * back to the original message.
 */
export function describeContractError(error: unknown): string {
  const code = parseContractErrorCode(error);
  if (code !== null) {
    const info = getContractErrorInfo(code);
    return `${info.title} — ${info.hint}`;
  }
  if (error instanceof Error && error.message) return error.message;
  if (typeof error === 'string' && error) return error;
  return 'Transaction failed. Please try again.';
}
