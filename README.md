🧠 #Solana Last Bid Program

A decentralized game-like smart contract on Solana where players buy "keys" using $BID tokens to extend a countdown timer. The last player to buy before the timer ends wins the main prize. Features include referral bonuses, revenue sharing for key holders, and flexible game rounds.

🚀 Features
Auction-style game: Timer-based bidding.

Dynamic pricing: Key prices increase per purchase.

Referral system: Earn rewards by sharing your referral code.

Revenue sharing: Key holders can claim shared revenue.

Multi-round gameplay with cooldown between rounds.

Ownership & admin functions to initialize and manage games.

📂 #Program Structure
📌 Constants
Defines game parameters like:

Initial prices and pool values

Timing intervals (base, increment, rest)

Fee distribution percentages (buy, referral, treasury)

Game mechanics (token amounts, max timer, etc.)

🛠 #Functions
🔑 Ownership Management
initialize_ownership
Initializes the ownership account. Can only be done once.

transfer_ownership(new_owner)
Changes ownership to another account. Requires current owner authorization.

🧑‍🤝‍🧑 Key Holders Management
create_key_holder_account(group_number)
Creates an empty group for holding key holder data. Requires ownership privileges.

close_key_holder_account(group_number)
Closes the key holder account. Admin-only.

🎮 Game Lifecycle
initialize_game(bid_token_mint, treasury_wallet)
Starts a new game round. Valid only after a REST_TIME period from the last game. Transfers the initial prize pool and resets game parameters.

close_game
Administrative function to end an ongoing game. Does not distribute prizes.

🛒 Key Purchase Mechanics
buy_keys(group_number, suggested_amount, number_of_keys)
Main function for buying keys. Key mechanics:

Timer is extended.

$BID tokens are burned.

SOL is paid and split across treasury and chest.

Updates key holders and revenue records.

buy_keys_with_referral_code(group_number, ref_code, suggested_amount, number_of_keys)
Same as buy_keys, but includes referral bonus logic. Validates the referral code and sends a portion of the fee to the referrer.

🏷 Referral System
create_referral_account(ref_code)
Sets up a referral account for the user. Must be unique and can only be created once per user.

🏆 #Prize & Rewards Distribution
release_main_prize
Distributes the main prize pool:

To the last bidder if there are any key holders.

Else, transfers to the treasury. Requires ownership and valid timing.

claim_revenue(group_number)
Key holders can claim their revenue share after the game ends. The group number must be valid. Transfers SOL proportionally to the key holders based on keys held.

claim_referral_bonus(ref_code)
Referrers can claim earned bonuses after users buy keys using their code.

📊 Event Logs
OwnershipEvent

GameInitEvent

KeyPurchasedEvent

KeyPurchasedWithReferralEvent

MainPrizeEvent

RevenueEvent

ReferralBonusEvent

ReferralAccountCreatedEvent

These provide on-chain logs for UI or analytics integrations.

⚠️ Error Handling
Custom errors (via BidErrorCode) include:

Incorrect group numbers

Insufficient funds or tokens

Invalid referrals

Unauthorized access

Game already ended or not started

Prize already claimed

🔐 Security & Permissions
Most functions enforce ownership or role-based access.

Key interactions use SPL Token and System Program CPI securely.

Requires BID token burning and fee transfers for purchases.

🧪 Deployment Notes
Built using Anchor framework.

Constants should be fine-tuned for real deployment.

Integrate with frontend for referral tracking, group display, and game countdown.

