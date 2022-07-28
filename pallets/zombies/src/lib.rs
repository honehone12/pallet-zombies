#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		transactional,
		sp_runtime::traits::Hash,
		dispatch::DispatchResult,
		traits::{ Currency, ExistenceRequirement, Randomness },
	};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_io::hashing::blake2_128;

	#[cfg(feature = "std")]
	use frame_support::serde::{ Serialize, Deserialize };

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> 
			+ IsType<<Self as frame_system::Config>::Event>;
		type Currency: Currency<Self::AccountId>;
		type ZombieRandomness: Randomness<Self::Hash, Self::BlockNumber>;
		#[pallet::constant]
		type MaxZombieOwned: Get<u32>;
		#[pallet::constant]
		type MaxZombieLevel: Get<u32>;
	}

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T>
		= <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum Gender {
		Male,
		Female,
		NewType
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	#[codec(mel_bound())]
	pub struct Zombie<T: Config> {
		pub dna: [u8; 16],
		pub price: Option<BalanceOf<T>>,
		pub gender: Gender,
		pub owner: AccountOf<T>,
		pub level: u32
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	// https://docs.substrate.io/v3/runtime/storage
	// Learn more about declaring storage items:
	// https://docs.substrate.io/v3/runtime/storage#declaring-storage-items
	#[pallet::storage]
	#[pallet::getter(fn count_for_zombies)]
	pub(super) type CountForZombies<T: Config>
		= StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn zombies)]
	pub(super) type Zombies<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::Hash,
		Zombie<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn zombies_owned)]
	pub(super) type ZombiesOwned<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<T::Hash, T::MaxZombieOwned>,
		ValueQuery,
	>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		Created(T::AccountId, T::Hash),
		PriceSet(T::AccountId, T::Hash, Option<BalanceOf<T>>),
		Transfered(T::AccountId, T::AccountId, T::Hash),
		Bought(T::AccountId, T::AccountId, T::Hash, BalanceOf<T>),
		Battle(T::Hash, T::Hash, bool)
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		/// Errors should have helpful documentation associated with them.
		CountForZombiesOverflow,
		ExceedMaxZombieOwned,
		BuyerIsZombieOwner,
		TransferToSelf,
		ZombieAlreadyExists,
		ZombieNotExists,
		NotZombieOwner,
		ZombieNotForSale,
		ZombieBidPriceTooLow,
		NotEnoughBalance,
		BattleWithSelf,
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub zombies: Vec<(T::AccountId, [u8; 16], Gender)>
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> GenesisConfig<T> {
			GenesisConfig {
				zombies: vec![
					
				]
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for (acct, dna, gender) in &self.zombies {
				let _ = <Pallet<T>>::mint(
					acct,
					Some(dna.clone()),
					Some(gender.clone()),
					0
				);
			}
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1000))]
		pub fn create_zombie(origin: OriginFor<T>) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/v3/runtime/origins
			let caller = ensure_signed(origin)?;
			let zombie_id
				= Self::mint(&caller, None, None, 0)?;
	
			log::info!(
				"a zombie is born with ID: {:?}.",
				zombie_id
			);
			Self::deposit_event(Event::Created(caller, zombie_id));
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn crate_cheat_zombie(
			origin: OriginFor<T>,
			gender: Gender,
			level: u32
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let zombie_id = Self::mint(
				&caller,
				None,
				Some(gender),
				level
			)?;
			log::info!(
				"a cheat zombie is born with ID: {:?},",
				zombie_id
			);
			Self::deposit_event(Event::Created(caller, zombie_id));
			Ok(())
		}


		/// An example dispatchable that may throw a custom error.
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn set_price(
			origin: OriginFor<T>,
			zombie_id: T::Hash,
			new_price: Option<BalanceOf<T>>
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			ensure!(
				Self::is_zombie_owner(&zombie_id, &caller)?,
				<Error<T>>::NotZombieOwner
			);

			let mut zombie = Self::zombies(&zombie_id)
				.ok_or(<Error<T>>::ZombieNotExists)?;
			zombie.price = new_price.clone();
			<Zombies<T>>::insert(&zombie_id, zombie);
			log::info!(
				"ID: {:?} zombie is set price {:?}.",
				zombie_id,
				new_price,
			);
			Self::deposit_event(Event::PriceSet(caller, zombie_id, new_price));
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn transfer(
			origin: OriginFor<T>,
			to: T::AccountId,
			zombie_id: T::Hash
		) -> DispatchResult {
			let from = ensure_signed(origin)?;

			ensure!(
				Self::is_zombie_owner(&zombie_id, &from)?,
				<Error<T>>::NotZombieOwner
			);
			ensure!(
				from != to,
				<Error<T>>::TransferToSelf
			);

			let owned_by_to
				= <ZombiesOwned<T>>::get(&to);
			ensure!(
				owned_by_to.len() < T::MaxZombieOwned::get() as usize,
				<Error<T>>::ExceedMaxZombieOwned
			);

			Self::transfer_zombie_to(&zombie_id, &to)?;
			log::info!(
				"ID: {:?} zombie is transfered FROM: {:?} To: {:?}.",
				zombie_id,
				from,
				to,
			);
			Self::deposit_event(Event::Transfered(from, to, zombie_id));
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn buy_zombie(
			origin: OriginFor<T>,
			zombie_id: T::Hash,
			bid_price: BalanceOf<T>,
		) -> DispatchResult {
			let buyer = ensure_signed(origin)?;
			let zombie = Self::zombies(&zombie_id)
				.ok_or(<Error<T>>::ZombieNotExists)?;

			ensure!(
				buyer != zombie.owner,
				<Error<T>>::BuyerIsZombieOwner
			);

			if let Some(ask_price
			) = zombie.price {
				ensure!(
					ask_price <= bid_price,
					<Error<T>>::ZombieBidPriceTooLow
				);
			} else {
				Err(<Error<T>>::ZombieNotForSale)?
			}

			ensure!(
				T::Currency::free_balance(&buyer) >= bid_price,
				<Error<T>>::NotEnoughBalance
			);

			let owned_by_to
				= <ZombiesOwned<T>>::get(&buyer);
				
			ensure!(
				owned_by_to.len() < T::MaxZombieOwned::get() as usize,
				<Error<T>>::ExceedMaxZombieOwned
			);

			let seller = zombie.owner.clone();
			T::Currency::transfer(
				&buyer,
				&seller,
				bid_price,
				ExistenceRequirement::KeepAlive
			)?;
			log::info!(
				"transfered currency FROM: {:?} TO: {:?} AMOUNT: {:?}.",
				buyer,
				seller,
				bid_price
			);
			Self::transfer_zombie_to(&zombie_id, &buyer)?;
			log::info!(
				"transfered zombie ID: {:?} FROM: {:?} TO: {:?}.",
				zombie_id,
				seller,
				buyer,
			);
			Self::level_up_zombie(&zombie_id)?;
			Self::deposit_event(
				Event::Bought(buyer, seller, zombie_id, bid_price)
			);
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn bleed_zombie(
			origin: OriginFor<T>,
			parent_a: T::Hash,
			parent_b: T::Hash,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			ensure!(
				Self::is_zombie_owner(&parent_a, &caller)?,
				<Error<T>>::NotZombieOwner
			);
			ensure!(
				Self::is_zombie_owner(&parent_b, &caller)?,
				<Error<T>>::NotZombieOwner
			);

			let new_dna = Self::breed_dna(&parent_a, &parent_b)?;
			Self::mint(&caller, Some(new_dna), None, 0)?;
			Self::level_up_zombie(&parent_a)?;
			Self::level_up_zombie(&parent_b)?;
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn battle(
			origin: OriginFor<T>,
			challenger: T::Hash,
			opponent: T::Hash,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			ensure!(
				Self::is_zombie_owner(&challenger, &caller)?,
				<Error<T>>::NotZombieOwner
			);
			ensure!(
				challenger != opponent,
				<Error<T>>::BattleWithSelf
			);
			ensure!(
				!Self::is_zombie_owner(&opponent, &caller)?,
				<Error<T>>::BattleWithSelf
			);

			log::info!(
				"battle!! {:?} VS {:?} !!",
				challenger,
				opponent
			);
			let did_win = Self::is_winner(
				&challenger,
				&opponent,
				true
			)?;
			log::info!(
				"challenger won ?? --> RESULT: {:?}",
				did_win
			);
			if did_win {
				Self::level_up_zombie(&challenger)?;
			} else {
				Self::level_up_zombie(&opponent)?;
			}
			Self::deposit_event(Event::Battle(challenger, opponent, did_win));
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn train(
			origin: OriginFor<T>,
			trainee: T::Hash,
			trainer: T::Hash
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			ensure!(
				Self::is_zombie_owner(&trainee, &caller)?,
				<Error<T>>::NotZombieOwner,
			);
			ensure!(
				Self::is_zombie_owner(&trainer, &caller)?,
				<Error<T>>::NotZombieOwner,
			);

			log::info!(
				"training... {:?} with {:?}.",
				trainee,
				trainer
			);
			let is_training_hard = !Self::is_winner(
				&trainee,
				&trainer,
				false
			)?;
			if is_training_hard {
				Self::level_up_zombie(&trainee)?;
			}
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn gen_gender() -> Gender {
			let random
				= T::ZombieRandomness::random(&b"gender"[..]).0;
			match random.as_ref()[0] % 2 {
				0 => Gender::Male,
				7 => Gender::NewType,
				_ => Gender::Female,
			}
		}

		fn gen_dna() -> [u8; 16] {
			let payload = (
				T::ZombieRandomness::random(&b"dna"[..]).0,
				<frame_system::Pallet<T>>::extrinsic_index().unwrap_or_default(),
				<frame_system::Pallet<T>>::block_number(),
			);
			payload.using_encoded(blake2_128)
		}

		pub fn breed_dna(parent_a: &T::Hash, parent_b: &T::Hash
		) -> Result<[u8; 16], Error<T>> {
			let dna_a = Self::zombies(parent_a)
				.ok_or(<Error<T>>::ZombieNotExists)?
				.dna;
			let dna_b = Self::zombies(parent_b)
				.ok_or(<Error<T>>::ZombieNotExists)?
				.dna;
			let mut new_dna = Self::gen_dna();
			for i in 0..new_dna.len() {
				new_dna[i] = 
					(new_dna[i] & dna_a[i]) | (!new_dna[i] & dna_b[i]);
			}
			Ok(new_dna)
		}

		pub fn mint(
			owner: &T::AccountId,
			dna: Option<[u8; 16]>,
			gender: Option<Gender>,
			level: u32
		) -> Result<T::Hash, Error<T>> {
			let zombie: Zombie<T> = Zombie::<T> {
				dna: dna.unwrap_or_else(Self::gen_dna),
				price: None,
				gender: gender.unwrap_or_else(Self::gen_gender),
				owner: owner.clone(),
				level
			};
			let zombie_id = T::Hashing::hash_of(&zombie);
			let new_cnt = Self::count_for_zombies()
				.checked_add(1)
				.ok_or(<Error<T>>::CountForZombiesOverflow)?;

			ensure!(
				Self::zombies(&zombie_id) == None,
				<Error<T>>::ZombieAlreadyExists
			);

			<ZombiesOwned<T>>::try_mutate(&owner, |zombie_vec| {
				zombie_vec.try_push(zombie_id)
			}).map_err(|_| <Error<T>>::ExceedMaxZombieOwned)?;
			<Zombies<T>>::insert(zombie_id, zombie);
			<CountForZombies<T>>::put(new_cnt);
			Ok(zombie_id)
		}

		pub fn is_zombie_owner(zombie_id: &T::Hash, acct: &T::AccountId
		) -> Result<bool, Error<T>> {
			match Self::zombies(zombie_id) {
				Some(zombie) => Ok(zombie.owner == *acct),
				None => Err(<Error<T>>::ZombieNotExists)
			}
		}

		fn level_up_zombie(zombie_id: &T::Hash
		) -> Result<(), Error<T>> {
			match Self::zombies(&zombie_id) {
				Some(mut zombie) => {
					if zombie.level >= T::MaxZombieLevel::get() {
						//we can use level.checked_add(1).ok_or(err) 
						return Ok(())
					}

					if zombie.gender == Gender::NewType {
						zombie.level += 3;
					} else {
						zombie.level += 1;
					}
					<Zombies<T>>::insert(zombie_id, zombie);
					Ok(())
				},
				None => Err(<Error<T>>::ZombieNotExists)
			}
		}

		fn is_winner(
			challenger: &T::Hash,
			opponent: &T::Hash,
			burst_new_type: bool
		) -> Result<bool, Error<T>> {
			let zombie_challenger
				= <Zombies<T>>::get(&challenger)
				.ok_or(<Error<T>>::ZombieNotExists)?;
			let zombie_opponent
				= <Zombies<T>>::get(&opponent)
				.ok_or(<Error<T>>::ZombieNotExists)?;
			
			let mut challenger_level = zombie_challenger.level;
			let mut opponent_level = zombie_opponent.level;
			
			if burst_new_type {
				if zombie_challenger.gender == Gender::NewType {
					challenger_level *= 3;
				}
				if zombie_opponent.gender == Gender::NewType {
					opponent_level *= 3;
				}
			}
			
			if challenger_level != opponent_level {
				return Ok(challenger_level > opponent_level)
			} else if T::ZombieRandomness::random(&b"battle"[..])
			.0.as_ref()[0] % 2 == 0 {
				return Ok(true)
			}
			Ok(false)
		} 

		#[transactional]
		pub fn transfer_zombie_to(
			zombie_id: &T::Hash,
			to: &T::AccountId
		) -> Result<(), Error<T>> {
			let mut zombie = Self::zombies(&zombie_id)
				.ok_or(<Error<T>>::ZombieNotExists)?;
			let prev_owner = zombie.owner.clone();
			<ZombiesOwned<T>>::try_mutate(
				prev_owner,
				|owned| {
					if let Some(prev) = owned
					.iter()
					.position(|&id| id == *zombie_id) {
						owned.swap_remove(prev);
						return Ok(());
					}
					Err(())
				}
			).map_err(|_| <Error<T>>::ZombieNotExists)?;
			
			zombie.owner = to.clone();
			zombie.price = None;
			<Zombies<T>>::insert(zombie_id, zombie);
			<ZombiesOwned<T>>::try_mutate(to, |owned| {
				owned.try_push(*zombie_id)
			}).map_err(|_| <Error<T>>::ExceedMaxZombieOwned)?;
			Ok(())
		}
	}
}
