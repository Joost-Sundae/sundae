use scrypto::prelude::*;

#[derive(Debug, NonFungibleData, ScryptoSbor)]
struct AmountDue {
    amount: Decimal,
    interest_rate: Decimal,
}

#[derive(Debug, NonFungibleData, ScryptoSbor)]
struct LiquiditySupplier {
    lsu_amount: Decimal,
    entry_epoch: Epoch,
}

#[blueprint]
mod flashloanpool {
    enable_method_auth! {
        roles {
            admin => updatable_by: [OWNER];
            component => updatable_by: [OWNER];
        },
        methods {
            get_flashloan => PUBLIC;
            repay_flashloan => PUBLIC;
            owner_deposit_liquidity => restrict_to: [OWNER];
            owner_withdraw_liquidity => restrict_to: [OWNER];
            staker_deposit_lsu => PUBLIC;
            deposit_batch => PUBLIC;
            staker_withdraw_lsu => PUBLIC;
            update_supplier_info => restrict_to: [admin, OWNER, component];
            update_interest_rate => restrict_to: [admin, OWNER];
            // mint_admin_badge => restrict_to: [admin];
        }
    }

    struct Flashloanpool {
        owner_badge_address:ResourceAddress,
        admin_badge_address: ResourceAddress,
        liquidity_admin: Decimal,
        liquidity_interest: Decimal,
        liquidity_emissions: Decimal,
        liquidity_pool_vault: Vault,
        supplier_hashmap: HashMap<NonFungibleLocalId, Vec<Decimal>>,
        lsu_vault: Vault,
        lsu_nft: ResourceManager,
        lsu_nft_nr: u64,
        interest_rate: Decimal,
        transient_token: ResourceManager,
    }

    impl Flashloanpool {
        pub fn instantiate_flashloan_pool(owner_badge: Bucket) -> (Bucket, Bucket, Global<Flashloanpool>) {
            let (address_reservation, component_address) =
                Runtime::allocate_component_address(Runtime::blueprint_id());

            // Provision fungible resource and generate admin's badge
            // to support (co-)ownership
            // Mintable and burnable by anyone who owns an admin's badge
            let admin_badge: Bucket = ResourceBuilder::new_fungible(OwnerRole::None)
                .divisibility(DIVISIBILITY_NONE)
                .metadata(metadata! {
                    init {
                        "name" => "Sundae FLP Admin Badge", locked;
                        "symbol" => "FLP", locked;
                        "description" => "Sundae flash loan pool admin badge", locked;
                    }
                })
                .mint_roles(mint_roles! {
                    minter => rule!(require(owner_badge.resource_address()));
                    minter_updater => rule!(deny_all);
                })
                .burn_roles(burn_roles! {
                    burner => rule!(require(owner_badge.resource_address()));
                    burner_updater => rule!(deny_all);
                })
                .mint_initial_supply(1);

            // Provision transient non-fungible resource
            // to enforce flashloan repayment
            let transient_token: ResourceManager =
                ResourceBuilder::new_ruid_non_fungible::<AmountDue>(OwnerRole::None)
                    .metadata(metadata! {
                        roles {
                            metadata_setter => rule!(require(admin_badge.resource_address()));
                            metadata_setter_updater => rule!(deny_all);
                            metadata_locker => rule!(require(admin_badge.resource_address()));
                            metadata_locker_updater => rule!(deny_all);
                        },
                        init {
                            "name" => "Sundae Transient Token", locked;
                            "symbol" => "STT", locked;
                            "description" => "Flashloan transient token - amount due must be returned to burn this token", locked;
                        }
                    })
                    .mint_roles(mint_roles! {
                        minter => rule!(require(global_caller(component_address)));
                        minter_updater => rule!(deny_all);
                    })
                    .burn_roles(burn_roles! {
                        burner => rule!(require(global_caller(component_address)));
                        burner_updater => rule!(deny_all);
                    })
                    .deposit_roles(deposit_roles! {
                        depositor => rule!(deny_all);
                        depositor_updater => rule!(deny_all);
                    })
                    .create_with_no_initial_supply();

            // Provision non-fungible resource
            // serves as proof of lsu deposit
            let lsu_nft: ResourceManager =
                ResourceBuilder::new_integer_non_fungible::<LiquiditySupplier>(
                    OwnerRole::None,
                )
                .metadata(metadata! {
                    roles {
                        metadata_setter => rule!(require(owner_badge.resource_address()));
                        metadata_setter_updater => rule!(deny_all);
                        metadata_locker => rule!(require(owner_badge.resource_address()));
                        metadata_locker_updater => rule!(deny_all);
                    },
                    init {
                        "name" => "Sundae Proof of Supply", locked;
                        "symbol" => "SPS", locked;
                        "description" => "NFT that represents a user's proof of supply", locked;
                    }
                })
                .mint_roles(mint_roles! {
                    minter => rule!(require(global_caller(component_address)));
                    minter_updater => rule!(deny_all);
                })
                .burn_roles(burn_roles! {
                    burner => rule!(require(global_caller(component_address)));
                    burner_updater => rule!(deny_all);
                })
                .create_with_no_initial_supply();

            let flashloan_component: Global<Flashloanpool> = Self {
                owner_badge_address: owner_badge.resource_address(),
                admin_badge_address: admin_badge.resource_address(),
                liquidity_admin: dec!("0"),
                liquidity_interest: dec!("0"),
                liquidity_emissions: dec!("0"),
                liquidity_pool_vault: Vault::new(RADIX_TOKEN),
                supplier_hashmap: HashMap::new(),
                lsu_vault: Vault::new(RADIX_TOKEN),
                lsu_nft: lsu_nft,
                lsu_nft_nr: 0,
                interest_rate: dec!("0"),
                transient_token: transient_token,
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::Fixed(rule!(require(owner_badge.resource_address()))))
            .roles(roles! {
                admin => rule!(require(admin_badge.resource_address()));
                component => rule!(require(global_caller(component_address)));
            })
            .metadata(metadata! {
                roles {
                    metadata_setter => rule!(require(owner_badge.resource_address()));
                    metadata_setter_updater => rule!(deny_all);
                    metadata_locker => rule!(require(owner_badge.resource_address()));
                    metadata_locker_updater => rule!(deny_all);
                },
                init {
                    "name" => "Sundae: Flash loan pool", locked;
                    "description" => 
                        "Official Sundae 'XRD flash loan pool' component that
                        (1) offers a liquidity pool that collects XRD from staking rewards,
                        and (2) issues flash loans from the pool."
                        , locked;
                    "tags" => [
                        "Sundae",
                        "DeFi",
                        "Lend",
                        "Borrow",
                        "Supply",
                        "Stake",
                        "Flash_loan",
                        "Interest",
                        "Liquidity",
                        "Liquidity_pool"
                    ],
                    locked;
                }
            })
            .enable_component_royalties(component_royalties! {
                roles { 
                    royalty_setter => OWNER; 
                    royalty_setter_updater => OWNER; 
                    royalty_locker => OWNER;
                    royalty_locker_updater => OWNER; 
                    royalty_claimer => OWNER;
                    royalty_claimer_updater => OWNER; 
                },
                init {
                    get_flashloan => Xrd(1.into()), locked;
                    repay_flashloan => Xrd(1.into()), locked;
                    owner_deposit_liquidity => Xrd(1.into()), locked; 
                    owner_withdraw_liquidity => Xrd(1.into()), locked;
                    staker_deposit_lsu => Xrd(1.into()), locked;
                    deposit_batch => Free, locked;
                    staker_withdraw_lsu => Xrd(1.into()), locked; 
                    update_supplier_info => Xrd(1.into()), locked;
                    update_interest_rate => Xrd(1.into()), locked;
                }
            })
            .with_address(address_reservation)
            .globalize();

            info!("{} admin badge is provided to user", admin_badge.amount());

            return (owner_badge, admin_badge, flashloan_component);
        }

        pub fn get_flashloan(&mut self, amount: Decimal) -> (Bucket, Bucket) {
            // Ensure requested amount is positive
            // and is less than the total available amount
            assert!(amount > dec!("0"), "Please provide an amount larger than 0");
            assert!(
                amount <= self.liquidity_pool_vault.amount(),
                "Please request a loan amount smaller than {}",
                self.liquidity_pool_vault.amount()
            );
        
            // Log
            info!("Loan amount: {} XRD", amount);
            info!("Interest amount: {} %", self.interest_rate);
        
            // Mint transient token
            let transient_token_resource_manager: ResourceManager = self.transient_token;
            let transient_token: Bucket = transient_token_resource_manager.mint_ruid_non_fungible(AmountDue {
                amount: amount,
                interest_rate: self.interest_rate,
            });
            debug!(
                "Transient token data: {:?}",
                transient_token
                    .as_non_fungible()
                    .non_fungible::<AmountDue>()
                    .data()
            );
        
            // Withdraw loan amount
            let loan: Bucket = self.liquidity_pool_vault.take(amount);
        
            // Return transient token bucket and loan bucket
            return (transient_token, loan);
        }
        
        pub fn repay_flashloan(&mut self, mut repayment: Bucket, transient_token: Bucket) -> Bucket {
            // Ensure transient token is original
            assert_eq!(
                self.transient_token.address(),
                transient_token.resource_address(),
                "Please provide the transient token"
            );
        
            // Ensure only a single transient token is provided
            assert_eq!(
                transient_token.amount(),
                dec!("1"),
                "Please provide exactly one transient token"
            );
        
            // Ensure repayment is done in XRD
            assert!(
                repayment.resource_address() == RADIX_TOKEN,
                "Please provide XRD as repayment"
            );
        
            // Log
            info!("Repayment amount offered: {} XRD", repayment.amount());
        
            // Extract loan terms from transient token data
            let loan_data: AmountDue = transient_token
                .as_non_fungible()
                .non_fungible()
                .data();
            let loan_amount: Decimal = loan_data.amount;
            let interest_rate: Decimal = loan_data.interest_rate;
        
            // Calculate amount due
            let interest_amount: Decimal = loan_amount * interest_rate;
            let repayment_amount: Decimal = loan_amount + interest_amount;
        
            // Allocate the liquidity earnings
            self.liquidity_interest += interest_amount / dec!("2");
            self.liquidity_admin += interest_amount / dec!("2");
        
            // Log
            info!("Repayment amount required: {} XRD", &loan_amount);
            info!("Interest amount required: {} XRD", &interest_amount);
        
            // Ensure at least full repayment amount is offered
            assert!(
                repayment.amount() >= repayment_amount,
                "Please provide at least the full amount back"
            );
        
            // Deposit repayment
            self.liquidity_pool_vault.put(repayment.take(repayment_amount));
        
            // Burn transient token
            transient_token.burn();
        
            // Return residual
            return repayment;
        }
        
        pub fn owner_deposit_liquidity(&mut self, deposit: Bucket) {
            // Ensure requested amount is positive
            // and is less than the total available amount
            assert!(
                deposit.amount() > dec!("0"),
                "Please deposit an amount larger than 0"
            );
        
            assert_eq!(
                deposit.resource_address(),
                RADIX_TOKEN,
                "Please deposit XRD"
            );
        
            // Log
            info!("Admin liquidity before deposit: {} XRD", self.liquidity_admin);
            info!("Admin liquidity provided: {} XRD", deposit.amount());
        
            // Administer liquidity amount provided by admin
            self.liquidity_admin += deposit.amount();
        
            // Deposit admin liquidity
            self.liquidity_pool_vault.put(deposit);
        
            // Log
            info!("Admin liquidity after deposit: {} XRD", self.liquidity_admin);
        }
        
        pub fn owner_withdraw_liquidity(&mut self, amount: Decimal) -> Bucket {
            // Ensure amount is positive
            assert!(
                amount > dec!("0"),
                "Please withdraw an amount larger than 0"
            );
        
            // Ensure amount is less or equal to liquidity provided by admin
            assert!(
                amount <= self.liquidity_pool_vault.amount(),
                "Please withdraw an amount smaller than or equal to {}",
                self.liquidity_admin
            );
        
            // Log
            info!("Admin liquidity before withdrawal: {} XRD", self.liquidity_admin);
            info!("Admin liquidity withdrawn: {} XRD", amount);
        
            debug!("{:?}", self.supplier_hashmap);
        
            // Update the suppliers hashmap before returning earnings
            self.update_supplier_info();
        
            debug!("{:?}", self.supplier_hashmap);
        
            // Subtract withdrawn amount from admin liquidity
            self.liquidity_admin -= amount;
        
            // Log
            info!("Admin liquidity after withdrawal: {} XRD", self.liquidity_admin);
        
            // Withdraw amount
            let withdraw: Bucket = self.liquidity_pool_vault.take(amount);
        
            // Return bucket
            return withdraw;
        }        

        // Temporary: Replicating validator node's staking rewards collection
        pub fn deposit_batch(&mut self, bucket: Bucket) {
            // Deposit batch into the liquidity pool vault
            self.liquidity_pool_vault.put(bucket);
        }

        pub fn staker_deposit_lsu(&mut self, lsu_tokens: Bucket) -> Bucket {
            // Ensure LSU tokens are provided
            assert_eq!(
                lsu_tokens.resource_address(),
                self.lsu_vault.resource_address(),
                "Please provide liquids staking units (LSU's) generated by the Sundae validator node"
            );

            // Ensure LSU's provided is greater than zero
            assert!(
                lsu_tokens.amount() > dec!("0"),
                "Please provide an amount of liquids staking units (LSU's) greater than zero"
            );

            // Log the number of LSU's provided
            info!("{} LSU's are provided", lsu_tokens.amount());

            // Update the suppliers hashmap (distribute earnings) before adding the current deposit to the hashmap
            debug!("{:?}", self.supplier_hashmap);
            self.update_supplier_info();
            debug!("{:?}", self.supplier_hashmap);

            // Increase the LSU local id by 1
            self.lsu_nft_nr += 1;

            // Get the current epoch
            let epoch: Epoch = Runtime::current_epoch();

            // Mint an NFT containing the deposited vector <lsu amount, epoch>
            let lsu_nft_resource_manager = self.lsu_nft;
            let lsu_nft: Bucket = lsu_nft_resource_manager.mint_non_fungible(
                &NonFungibleLocalId::Integer(self.lsu_nft_nr.into()),
                LiquiditySupplier {
                    lsu_amount: lsu_tokens.amount(),
                    entry_epoch: epoch,
                },
            );

            // Determine variables for the vector in the supplier hashmap
            let lsu_amount: Decimal = lsu_tokens.amount() as Decimal;
            let staking_rewards: Decimal = dec!("0");
            let interest_earnings: Decimal = dec!("0");

            // Insert variables into the vector
            let lsu_nft_data: Vec<Decimal> = vec![lsu_amount, staking_rewards, interest_earnings];

            // Insert NFT local id as key and vector as value into the supplier hashmap
            self.supplier_hashmap
                .insert(NonFungibleLocalId::Integer(self.lsu_nft_nr.into()), lsu_nft_data);

            // Put provided LSU tokens in the LSU vault
            self.lsu_vault.put(lsu_tokens);

            debug!("{:?}", self.supplier_hashmap);

            // Return NFT as proof of LSU deposit to the user
            return lsu_nft;
        }

        pub fn staker_withdraw_lsu(&mut self, lsu_nft: Bucket) -> (Bucket, Bucket) {
            // Ensure LSU NFT is provided
            assert_eq!(
                lsu_nft.resource_address(),
                self.lsu_nft.address(),
                "Please provide the proof of supply (LSU NFT) generated by the Sundae validator node"
            );

            assert_eq!(
                lsu_nft.amount(),
                dec!("1"),
                "Please provide only one NFT"
            );

            // Update the suppliers hashmap (distribute earnings) before returning LSU's and XRD earnings to the user
            // This ensures that the supplier receives the correct amount of resources they are entitled to
            debug!("{:?}", self.supplier_hashmap);
            self.update_supplier_info();
            debug!("{:?}", self.supplier_hashmap);

            // Get the local id of the provided NFT, which resembles the key in the supplier hashmap
            let lsu_nft_nr = lsu_nft
                .as_non_fungible()
                .non_fungible_local_id();

            debug!("{:?}", lsu_nft_nr);

            debug!("{:?}", self.supplier_hashmap[&lsu_nft_nr]);

            // Withdraw entitled LSU's and earnings from vaults and return as a bucket
            let lsu_bucket: Bucket = self.lsu_vault.take(self.supplier_hashmap[&lsu_nft_nr][0]);
            let earnings: Decimal = self.supplier_hashmap[&lsu_nft_nr][1] + self.supplier_hashmap[&lsu_nft_nr][2];
            let earnings_bucket: Bucket = self.liquidity_pool_vault.take(earnings);

            // Log the LSU's and earnings returned to the supplier
            info!(
                "{} LSU's and {} XRD is returned to the supplier",
                lsu_bucket.amount(),
                earnings
            );

            // Remove the supplier's entry from the supplier hashmap
            self.supplier_hashmap.remove(&lsu_nft_nr);

            debug!("{:?}", self.supplier_hashmap);

            // Burn the provided NFT
            lsu_nft.burn();

            // Return LSU's and rewards to the user
            return (lsu_bucket, earnings_bucket);
        }

        pub fn update_supplier_info(&mut self) {
            // Log pool liquidity, admin liquidity, and supplier hashmap
            info!("Pool liquidity: {} XRD", self.liquidity_pool_vault.amount());
            info!("Admin liquidity: {} XRD", self.liquidity_admin);
            debug!("{:?}", self.supplier_hashmap);

            // Calculate the undistributed supplier earnings (staking rewards) that have to be distributed:
            // total liquidity = admin liquidity (seed liquidity + interest earnings)
            //                   + supplier distributed earnings (staking rewards + interest earnings)
            //                   + supplier undistributed earnings (staking rewards + interest earnings)
            // supplier undistributed earnings = total liquidity - admin liquidity - supplier distributed earnings

            // Determine 'total liquidity'
            let total_liquidity: Decimal = self.liquidity_pool_vault.amount();

            // Determine 'admin liquidity'
            let admin_liquidity: Decimal = self.liquidity_admin;

            // Determine 'supplier distributed earnings' by summing the rewards in the hashmap
            let supplier_distributed_rewards: Decimal = self
                .supplier_hashmap
                .values()
                .filter_map(|i| i.get(1))
                .copied()
                .sum();

            info!("Supplier distributed rewards: {} XRD", supplier_distributed_rewards);

            let supplier_distributed_interest: Decimal = self
                .supplier_hashmap
                .values()
                .filter_map(|i| i.get(2))
                .copied()
                .sum();

            info!("Supplier distributed interest: {} XRD", supplier_distributed_interest);

            let supplier_distributed_earnings: Decimal =
                supplier_distributed_rewards + supplier_distributed_interest;

            info!("Supplier distributed earnings: {} XRD", supplier_distributed_earnings);

            let supplier_undistributed_interest: Decimal = self.liquidity_interest;

            info!("Supplier undistributed interest: {} XRD", supplier_undistributed_interest);

            // Calculate the suppliers (undistributed) earnings
            let supplier_undistributed_rewards: Decimal = total_liquidity
                - admin_liquidity
                - supplier_distributed_earnings
                - supplier_undistributed_interest;

            info!("Supplier undistributed rewards: {} XRD", supplier_undistributed_rewards);

            // Loop over all entries in the hashmap to update information
            for i in self.supplier_hashmap.values_mut() {
                // Determine supplier's LSU stake
                let supplier_lsu = i[0];
                // Determine supplier's XRD stake
                let supplier_xrd = i[1] + i[2];

                // Determine supplier's LSU stake relative to the pool's total LSU
                let supplier_relative_lsu_stake = supplier_lsu / self.lsu_vault.amount();

                let supplier_relative_xrd_stake = if supplier_distributed_interest != dec!("0") {
                    // Determine supplier's XRD stake relative to the distributed earnings
                    supplier_xrd / supplier_distributed_earnings
                } else {
                    // Handle the case where `supplier_distributed_interest` is zero
                    // Assign a default value (`supplier_relative_lsu_stake`)
                    supplier_relative_lsu_stake
                };

                // Distribute the new staking rewards based on the staker's LSU relative to the pool's total LSU
                i[1] += supplier_undistributed_rewards * supplier_relative_lsu_stake;

                // Distribute the new interest earnings based on the staker's XRD relative to the pool's total XRD
                i[2] += supplier_undistributed_interest * supplier_relative_xrd_stake;
            }

            self.liquidity_interest = dec!("0");

            debug!("{:?}", self.supplier_hashmap);
        }

        pub fn update_interest_rate(&mut self, interest_rate: Decimal) {
            // Ensure interest rate is larger than 0
            assert!(
                interest_rate >= dec!("0"),
                "Please provide an interest rate larger than 0"
            );

            // Log the interest rate before and after change
            info!("Interest rate before change: {}", self.interest_rate);
            self.interest_rate = interest_rate;
            info!("Interest rate after change: {}", self.interest_rate);
        }
    }
}

// pub fn mint_admin_badge(&mut self) -> Bucket {
//     // Mint an admin badge
//     let admin_badge_resource_manager: ResourceManager = self.admin_badge_address.into();
//     let admin_badge: Bucket = admin_badge_resource_manager.mint(1);
//     return admin_badge;
// }