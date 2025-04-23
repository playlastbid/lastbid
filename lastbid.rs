use anchor_lang::{prelude::*, system_program};
use anchor_spl::token;

pub mod utils;
pub use utils::*;

declare_id!("77iKeKrz9xHSzyPHqP3haspcigK8kQARBk6NsFnWmp9j");

#[program]
pub mod solana_lastbid_program {
    use super::*;

    // const INITIAL_KEY_PRICE: u64 = 10_000_000; // 0.01 SOL
    // const INITIAL_PRIZE_POOL: u64 = 25_000_000; // 0.025 SOL 
    // const BASE_TIMER: i64 = 300; // 5 minutes
    // const INC_TIME: i64 = 30; // 30 seconds
    // const MAX_TIMER: i64 = 28800; // 8 hours
    // const REST_TIME: i64 = 120; // 2 minutes
    // const PRICE_INCREASE_RATE: u64 = 5; // 0.05 %
    // const PRICE_TOLERANCE: u64 = 1000; // 10 %
    // const BID_TOKENS_PER_TX: u64 = 10_000_000_000; // 10 $BID per transaction, decimal: 9

    // const BUY_FEE: u64 = 100; // 1 %
    // const DISTRIBUTION_FEE: u64 = 250; // 2.5 %
    // const LAST_BIDDER_SHARE: u64 = 6000; // 60 %
    // const KEY_HOLDERS_SHARE: u64 = 4000; // 40 %
    // const REFERRAL_SHARE: u64 = 2000; // 20%

    const INITIAL_KEY_PRICE: u64 = 10_000_000; // 0.01 SOL
    const INITIAL_PRIZE_POOL: u64 = 25_000_000_000; // 25 SOL 
    const BASE_TIMER: i64 = 900; // 15 minutes
    const INC_TIME: i64 = 900; // 15 minutes
    const MAX_TIMER: i64 = 28800; // 8 hours
    const REST_TIME: i64 = 43200; // 12 hours
    const PRICE_INCREASE_RATE: u64 = 10; // 0.1%
    const PRICE_TOLERANCE: u64 = 1000; // 10%
    const BID_TOKENS_PER_TX: u64 = 25_000_000; // 25 $BID per transaction, decimal: 6

    const BUY_FEE: u64 = 100; // 1%
    const DISTRIBUTION_FEE: u64 = 300; // 3%
    const LAST_BIDDER_SHARE: u64 = 6000; // 60%
    const KEY_HOLDERS_SHARE: u64 = 4000; // 40%
    const REFERRAL_SHARE: u64 = 2000; // 20%

    const DIVIDER: u64 = 10000;

    pub fn initialize_ownership(ctx: Context<InitializeOwnership>) -> Result<()> {
        let ownership = &mut ctx.accounts.ownership;
        let clock = Clock::get()?;

        require!(
            !ownership.initialized,
            BidErrorCode::OwnershipAlreadyInitialized
        );

        ownership.owner = ctx.accounts.owner.key();
        ownership.timestamp = clock.unix_timestamp;
        ownership.initialized = true;

        emit!(OwnershipEvent {
            new_owner: ownership.owner,
            timestamp: ownership.timestamp,
        });

        Ok(())
    }

    pub fn transfer_ownership(ctx: Context<TransferOwnership>, new_owner: Pubkey) -> Result<()> {
        let ownership = &mut ctx.accounts.ownership;
        let clock = Clock::get()?;

        ownership.verify_ownership(ctx.accounts.owner.key())?;

        ownership.owner = new_owner;
        ownership.timestamp = clock.unix_timestamp;

        emit!(OwnershipEvent {
            new_owner,
            timestamp: ownership.timestamp,
        });

        Ok(())
    }

    pub fn create_key_holder_account(
        ctx: Context<CreateKeyHolderAccount>,
        group_number: u64,
    ) -> Result<()> {
        let ownership = &mut ctx.accounts.ownership;
        let key_holders = &mut ctx.accounts.key_holders;

        ownership.verify_ownership(ctx.accounts.owner.key())?;

        key_holders.group_number = group_number;
        key_holders.holders = vec![];

        Ok(())
    }

    pub fn close_key_holder_account(ctx: Context<CloseKeyHolderAccount>, _group_number: u64) -> Result<()> {
        let ownership = &ctx.accounts.ownership;
        ownership.verify_ownership(ctx.accounts.owner.key())?;
        Ok(())
    }

    pub fn initialize_game(
        ctx: Context<InitializeGame>,
        bid_token_mint: Pubkey,
        treasury_wallet: Pubkey,
    ) -> Result<()> {
        let game: &mut Account<'_, Game> = &mut ctx.accounts.game;
        let ownership = &ctx.accounts.ownership;
        let group_revenue_counter = &mut ctx.accounts.group_revenue_counter;
        let clock = Clock::get()?;

        // Verify the ownership
        ownership.verify_ownership(ctx.accounts.payer.key())?;

        // The game can be initialized 10(REST_TIME) hours later after the last game ended
        require!(
            game.timer_end == 0 || game.timer_end + REST_TIME < clock.unix_timestamp,
            BidErrorCode::RestNotFinished
        );

        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.payer.to_account_info().clone(),
                    to: ctx.accounts.chest_vault.to_account_info().clone(),
                },
            ),
            INITIAL_PRIZE_POOL,
        )?;

        game.game_id = game.game_id + 1;
        game.owner = ctx.accounts.payer.key();
        game.last_bidder = Pubkey::default();
        game.treasury = treasury_wallet;
        game.bid_token_mint = bid_token_mint;
        game.current_price = INITIAL_KEY_PRICE;
        game.prize_pool_balance = INITIAL_PRIZE_POOL + game.revenue_earned;
        game.revenue_earned = 0;
        game.last_purchase_time = 0;
        game.timer_end = clock.unix_timestamp + BASE_TIMER;
        game.total_keys = 0;
        game.total_amount = 0;
        game.total_groups = 0;
        game.total_holders = 0;
        game.active = true;
        game.prized = false;
        // game.last_chainlink_timestamp = get_chainlink_timestamp(&ctx.accounts.chainlink_feed)?;

        group_revenue_counter.group_counter = vec![];

        emit!(GameInitEvent {
            owner: game.owner,
            timestamp: clock.unix_timestamp,
            timer_end: game.timer_end,
        });

        Ok(())
    }

    pub fn close_game(ctx: Context<CloseGame>) -> Result<()> {
        let ownership = &ctx.accounts.ownership;
        ownership.verify_ownership(ctx.accounts.owner.key())?;
        Ok(())
    }

    pub fn buy_keys(
        ctx: Context<BuyKeys>,
        group_number: u64,
        suggested_amount: u64,
        number_of_keys: u64,
    ) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let key_holders = &mut ctx.accounts.key_holders;
        let buyer_game_data = &mut ctx.accounts.buyer_game_account;
        let group_revenue_data = &mut ctx.accounts.group_revenue_counter;
        let clock = Clock::get()?;

        // Verify game is active
        require!(
            game.active && clock.unix_timestamp < game.timer_end,
            BidErrorCode::GameEnded
        );

        // Init buyer game data
        if buyer_game_data.game_id != game.game_id {
            buyer_game_data.game_id = game.game_id;
            buyer_game_data.first_time_buying = true;
        }

        // Verify group number
        let current_group_number = game.total_holders / (MAX_HOLDERS as u64);
        require!(
            group_number <= current_group_number,
            BidErrorCode::IncorrectGroupNumber
        );

        if game.total_holders == game.total_groups * (MAX_HOLDERS as u64) {
            // if the buyer is the first person to buy keys in this group, then init the key_holders data
            if current_group_number == group_number {
                group_revenue_data.group_counter.push(0);
                key_holders.holders = vec![];
                game.total_groups += 1;
            }
        }

        let already_exist = check_if_buyer_is_in_the_group(key_holders, ctx.accounts.buyer.key())?;
        require!(
            (group_number < current_group_number && already_exist) || !(group_number
                == current_group_number
                && !already_exist
                && !buyer_game_data.first_time_buying),
            BidErrorCode::IncorrectGroupNumber
        );

        // Verify the treasury wallet
        require_keys_eq!(
            ctx.accounts.treasury.key(),
            game.treasury,
            BidErrorCode::NotTreasury
        );

        let extensible_time = if clock.unix_timestamp + MAX_TIMER >= game.timer_end {
            clock.unix_timestamp + MAX_TIMER - game.timer_end
        } else {
            0
        };
        let available_keys = std::cmp::min(number_of_keys, (extensible_time / INC_TIME) as u64);
        let new_end_time = game.timer_end + INC_TIME * (available_keys as i64);

        // Verify BID token balance to burn
        require!(
            game.bid_token_mint == ctx.accounts.bid_token_mint_account.key(),
            BidErrorCode::IncorrectBidToken
        );
        let bid_token_balance = ctx.accounts.buyer_bid_token_account.amount;
        require!(
            bid_token_balance >= BID_TOKENS_PER_TX * available_keys,
            BidErrorCode::InsufficientBidTokens
        );

        let data: Fees = calculate_fees_and_next_price(
            BUY_FEE,
            REFERRAL_SHARE,
            LAST_BIDDER_SHARE,
            KEY_HOLDERS_SHARE,
            available_keys,
            game.current_price,
            PRICE_INCREASE_RATE,
            DIVIDER,
            false,
        )?;

        let max_amount = suggested_amount * (DIVIDER + PRICE_TOLERANCE) / DIVIDER;
        require!(
            max_amount >= data.total_amount,
            BidErrorCode::InvalidPaymentAmount
        );

        // transfer sol from buyer wallet to chest
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.buyer.to_account_info().clone(),
                    to: ctx.accounts.chest_vault.to_account_info().clone(),
                },
            ),
            data.total_amount - data.treasury_amount,
        )?;

        // transfer buy fee from buyer wallet to treasury
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.buyer.to_account_info().clone(),
                    to: ctx.accounts.treasury.to_account_info().clone(),
                },
            ),
            data.treasury_amount,
        )?;

        // Burn $BID tokens
        token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Burn {
                    mint: ctx
                        .accounts
                        .bid_token_mint_account
                        .to_account_info()
                        .clone(),
                    from: ctx
                        .accounts
                        .buyer_bid_token_account
                        .to_account_info()
                        .clone(),
                    authority: ctx.accounts.buyer.to_account_info().clone(),
                },
            ),
            BID_TOKENS_PER_TX * available_keys,
        )?;

        adjust_revenue(
            ctx.accounts.buyer.key(),
            key_holders,
            group_revenue_data,
            data.key_holders_amount,
            group_number,
            game.total_keys,
        )?;

        update_key_holders(
            key_holders,
            ctx.accounts.buyer.key(),
            available_keys,
            clock.unix_timestamp,
        )?;

        if buyer_game_data.first_time_buying {
            game.total_holders += 1;
            buyer_game_data.first_time_buying = false;
        }

        // Update game state
        game.last_bidder = ctx.accounts.buyer.key();
        game.total_keys += available_keys;
        game.total_amount += data.total_amount;
        game.current_price = data.next_key_price;
        game.prize_pool_balance += data.prize_pool_amount;
        game.last_purchase_time = clock.unix_timestamp;
        game.timer_end = new_end_time;

        // adjust revenue of the previous key_holders
        game.revenue_earned += data.key_holders_amount;

        emit!(KeyPurchasedEvent {
            game_id: game.game_id,
            buyer: ctx.accounts.buyer.key(),
            amount: data.total_amount,
            number_of_keys: available_keys,
            new_price: game.current_price,
            timer_end: game.timer_end,
            purchased_at: clock.unix_timestamp,
        });

        Ok(())
    }

    pub fn buy_keys_with_referral_code(
        ctx: Context<BuyKeysWithReferralCode>,
        group_number: u64,
        ref_code: String,
        suggested_amount: u64,
        number_of_keys: u64,
    ) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let key_holders = &mut ctx.accounts.key_holders;
        let buyer_game_data = &mut ctx.accounts.buyer_game_account;
        let group_revenue_data = &mut ctx.accounts.group_revenue_counter;
        let referral_data = &mut ctx.accounts.referral_account;
        let clock = Clock::get()?;

        // Verify game is active
        require!(
            game.active && clock.unix_timestamp < game.timer_end,
            BidErrorCode::GameEnded
        );

        // Init buyer game data
        if buyer_game_data.game_id != game.game_id {
            buyer_game_data.game_id = game.game_id;
            buyer_game_data.first_time_buying = true;
        }

        // Verify group number
        let current_group_number = game.total_holders / (MAX_HOLDERS as u64);
        require!(
            group_number <= current_group_number,
            BidErrorCode::IncorrectGroupNumber
        );

        if game.total_holders == game.total_groups * (MAX_HOLDERS as u64) {
            // if the buyer is the first person to buy keys in this group, then init the key_holders data
            if current_group_number == group_number {
                group_revenue_data.group_counter.push(0);
                key_holders.holders = vec![];
                game.total_groups += 1;
            }
        }

        let already_exist = check_if_buyer_is_in_the_group(key_holders, ctx.accounts.buyer.key())?;
        require!(
            (group_number < current_group_number && already_exist) || !(group_number
                == current_group_number
                && !already_exist
                && !buyer_game_data.first_time_buying),
            BidErrorCode::IncorrectGroupNumber
        );

        // Verify the treasury wallet
        require_keys_eq!(
            ctx.accounts.treasury.key(),
            game.treasury,
            BidErrorCode::NotTreasury
        );

        // Verify the referrer data
        require!(
            referral_data.owner.key() != ctx.accounts.buyer.key()
                && referral_data.ref_code == ref_code
                && referral_data.active,
            BidErrorCode::IncorrectReferralData
        );

        let extensible_time = if clock.unix_timestamp + MAX_TIMER >= game.timer_end {
            clock.unix_timestamp + MAX_TIMER - game.timer_end
        } else {
            0
        };
        let available_keys = std::cmp::min(number_of_keys, (extensible_time / INC_TIME) as u64);
        let new_end_time = game.timer_end + INC_TIME * (available_keys as i64);

        // Verify BID token balance to burn
        require!(
            game.bid_token_mint == ctx.accounts.bid_token_mint_account.key(),
            BidErrorCode::IncorrectBidToken
        );
        let bid_token_balance = ctx.accounts.buyer_bid_token_account.amount;
        require!(
            bid_token_balance >= BID_TOKENS_PER_TX * available_keys,
            BidErrorCode::InsufficientBidTokens
        );

        let data = calculate_fees_and_next_price(
            BUY_FEE,
            REFERRAL_SHARE,
            LAST_BIDDER_SHARE,
            KEY_HOLDERS_SHARE,
            available_keys,
            game.current_price,
            PRICE_INCREASE_RATE,
            DIVIDER,
            true,
        )?;

        let max_amount = suggested_amount * (DIVIDER + PRICE_TOLERANCE) / DIVIDER;
        require!(
            max_amount >= data.total_amount,
            BidErrorCode::InvalidPaymentAmount
        );

        // transfer sol from buyer wallet to chest
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.buyer.to_account_info().clone(),
                    to: ctx.accounts.chest_vault.to_account_info().clone(),
                },
            ),
            data.total_amount - data.treasury_amount,
        )?;
        // transfer buy fee from buyer wallet to treasury
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.buyer.to_account_info().clone(),
                    to: ctx.accounts.treasury.to_account_info().clone(),
                },
            ),
            data.treasury_amount,
        )?;

        // Burn $BID tokens
        token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Burn {
                    mint: ctx
                        .accounts
                        .bid_token_mint_account
                        .to_account_info()
                        .clone(),
                    from: ctx
                        .accounts
                        .buyer_bid_token_account
                        .to_account_info()
                        .clone(),
                    authority: ctx.accounts.buyer.to_account_info().clone(),
                },
            ),
            BID_TOKENS_PER_TX * available_keys,
        )?;

        adjust_revenue(
            ctx.accounts.buyer.key(),
            key_holders,
            group_revenue_data,
            data.key_holders_amount,
            group_number,
            game.total_keys,
        )?;

        update_key_holders(
            key_holders,
            ctx.accounts.buyer.key(),
            available_keys,
            clock.unix_timestamp,
        )?;

        if buyer_game_data.first_time_buying {
            game.total_holders += 1;
            buyer_game_data.first_time_buying = false;
        }

        // Update game state
        game.last_bidder = ctx.accounts.buyer.key();
        game.total_keys += available_keys;
        game.total_amount += data.total_amount;
        game.current_price = data.next_key_price;
        game.prize_pool_balance += data.prize_pool_amount;
        game.last_purchase_time = clock.unix_timestamp;
        game.timer_end = new_end_time;

        // update the referrer data
        referral_data.total_earned += data.referral_amount;
        game.referral_earned += data.referral_amount;

        // adjust revenue of the previous key_holders
        game.revenue_earned += data.key_holders_amount;

        emit!(KeyPurchasedWithReferralEvent {
            game_id: game.game_id,
            buyer: ctx.accounts.buyer.key(),
            ref_code: referral_data.ref_code.clone(),
            amount: data.total_amount,
            number_of_keys: available_keys,
            new_price: game.current_price,
            timer_end: game.timer_end,
            purchased_at: clock.unix_timestamp,
        });

        Ok(())
    }

    pub fn create_referral_account(
        ctx: Context<CreateReferralAccount>,
        ref_code: String,
    ) -> Result<()> {
        let referral_account = &mut ctx.accounts.referral_account;

        require!(
            !referral_account.active,
            BidErrorCode::AlreadyActivedReferralAccount
        );

        referral_account.active = true;
        referral_account.owner = ctx.accounts.payer.key();
        referral_account.ref_code = ref_code;
        referral_account.total_earned = 0;
        referral_account.created_at = Clock::get()?.unix_timestamp;

        emit!(ReferralAccountCreatedEvent {
            owner: referral_account.owner,
            ref_code: referral_account.ref_code.clone(),
            timestamp: referral_account.created_at,
        });

        Ok(())
    }

    pub fn release_main_prize(ctx: Context<ReleaseMainPrize>) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let ownership = &ctx.accounts.ownership;
        let chest_vault = &mut ctx.accounts.chest_vault;
        let clock = Clock::get()?;

        // Verify the ownership
        ownership.verify_ownership(ctx.accounts.owner.key())?;

        // Verify the treasury wallet
        require_keys_eq!(
            ctx.accounts.treasury.key(),
            game.treasury,
            BidErrorCode::NotTreasury
        );

        require!(
            clock.unix_timestamp >= game.timer_end,
            BidErrorCode::TimerNotExpired
        );

        require!(!game.prized, BidErrorCode::AlreadyPrized);

        game.active = false;

        if game.total_holders > 0 {
            if ctx.accounts.last_bidder.key() == game.last_bidder {
                game.prized = true;

                let prize_fee_amount = game.prize_pool_balance * DISTRIBUTION_FEE / DIVIDER;
                // transfer main prize fee to treasury
                transfer_sol(
                    chest_vault.to_account_info().clone(),
                    ctx.accounts.treasury.to_account_info(),
                    prize_fee_amount,
                )?;
                // transfer main prize to last bidder
                transfer_sol(
                    chest_vault.to_account_info().clone(),
                    ctx.accounts.last_bidder.to_account_info(),
                    game.prize_pool_balance - prize_fee_amount,
                )?;

                emit!(MainPrizeEvent {
                    winner: game.last_bidder,
                    amount: game.prize_pool_balance,
                    timestamp: clock.unix_timestamp,
                    new_round_start_at: game.timer_end + REST_TIME,
                });

                game.prize_pool_balance = 0;
            }
        } else {
            // if there is no any key_holders, it means there is no last bidder. So the prize goes to treasury wallet
            game.prized = true;

            transfer_sol(
                chest_vault.to_account_info().clone(),
                ctx.accounts.treasury.to_account_info(),
                game.prize_pool_balance,
            )?;

            emit!(MainPrizeEvent {
                winner: ctx.accounts.treasury.key(),
                amount: game.prize_pool_balance,
                timestamp: clock.unix_timestamp,
                new_round_start_at: game.timer_end + REST_TIME,
            });

            game.prize_pool_balance = 0;
        }

        Ok(())
    }

    pub fn claim_revenue(ctx: Context<ClaimRevenue>, group_number: u64) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let key_holders = &mut ctx.accounts.key_holders;
        let group_revenue_data = &mut ctx.accounts.group_revenue_counter;
        let clock = Clock::get()?;

        // Verify if the game ended
        // require!(
        //     !game.active && clock.unix_timestamp >= game.timer_end,
        //     BidErrorCode::GameNotEnded
        // );

        require!(game.total_holders > 0, BidErrorCode::NoKeyHolders);

        // Verify group number
        let current_group_number = game.total_holders / (MAX_HOLDERS as u64);
        require!(
            (group_number < current_group_number) || !(game.total_holders == game.total_groups * (MAX_HOLDERS as u64)
                && group_number == current_group_number),
            BidErrorCode::IncorrectGroupNumber
        );

        // Send payment to the payer and distribute group revenue to each holder based on their keys
        let mut revenue_claimed = false;
        let group_revenue = group_revenue_data.group_counter[group_number as usize];
        for holder in key_holders.holders.iter_mut() {
            if holder.holder != ctx.accounts.payer.key() {
                let holder_amount = group_revenue * holder.keys;
                holder.total_earned += holder_amount;
            } else {
                require!(
                    holder.total_earned > 0 && !holder.claimed,
                    BidErrorCode::NoRevenue
                );

                let revenue_amount = holder.total_earned + holder.keys * group_revenue;
                let amount =
                    if ctx.accounts.chest_vault.to_account_info().lamports() > revenue_amount {
                        revenue_amount
                    } else {
                        ctx.accounts.chest_vault.to_account_info().lamports()
                    };

                game.revenue_earned -= revenue_amount;
                holder.claimed = true;
                holder.total_earned = 0;
                revenue_claimed = true;

                transfer_sol(
                    ctx.accounts.chest_vault.to_account_info().clone(),
                    ctx.accounts.payer.to_account_info(),
                    amount,
                )?;

                emit!(RevenueEvent {
                    key_holder: holder.holder,
                    claimed_revenue: amount,
                    timestamp: clock.unix_timestamp,
                });
            }
        }
        group_revenue_data.group_counter[group_number as usize] = 0;

        require!(revenue_claimed, BidErrorCode::PayerNotInKeyHolders);

        Ok(())
    }

    pub fn claim_referral_bonus(ctx: Context<ClaimReferralBonus>, _ref_code: String) -> Result<()> {
        let referral_data = &mut ctx.accounts.referral_account;
        let game = &mut ctx.accounts.game;
        let clock = Clock::get()?;

        // Verify if the referral account actived or the payer is the owner of referral account
        require!(
            referral_data.active && referral_data.owner == ctx.accounts.payer.key(),
            BidErrorCode::IncorrectReferralData
        );
        require!(
            referral_data.total_earned > 0,
            BidErrorCode::NoReferralBonus
        );

        let amount =
            if ctx.accounts.chest_vault.to_account_info().lamports() > referral_data.total_earned {
                referral_data.total_earned
            } else {
                ctx.accounts.chest_vault.to_account_info().lamports()
            };

        game.referral_earned -= referral_data.total_earned;
        referral_data.total_earned = 0;

        transfer_sol(
            ctx.accounts.chest_vault.to_account_info().clone(),
            ctx.accounts.payer.to_account_info(),
            amount,
        )?;

        emit!(ReferralBonusEvent {
            referrer: referral_data.owner,
            claimed_amount: amount,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }
}
