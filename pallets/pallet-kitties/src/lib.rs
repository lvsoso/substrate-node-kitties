#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// https://substrate.dev/docs/en/knowledgebase/runtime/frame

use frame_support::{decl_module, decl_storage, decl_event, decl_error, dispatch, traits::Get,
					ensure, sp_runtime, Parameter};
use frame_system::ensure_signed;
use sp_runtime::DispatchError;
use frame_support::traits::Randomness;
use frame_support::traits::Currency;
use frame_support::traits::ReservableCurrency;

use codec::{Encode, Decode};
use sp_io::hashing::blake2_128;
use sp_runtime::traits::{AtLeast32Bit, Bounded, Member};

use sp_std::vec;


#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[derive(Encode, Decode)]
pub struct Kitty(pub [u8; 16]);

type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type Randomness: Randomness<Self::Hash>;
	type KittyIndex: Parameter + Member + AtLeast32Bit + Bounded + Default + Copy;

	type NewKittyReserve: Get<BalanceOf<Self>>;
	type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

}

// The pallet's runtime storage items.
// https://substrate.dev/docs/en/knowledgebase/runtime/storage
decl_storage! {
	trait Store for Module<T: Trait> as KittiesModule {
		// Learn more about declaring storage items:
		// https://substrate.dev/docs/en/knowledgebase/runtime/storage#declaring-storage-items
		Something get(fn something): Option<u32>;
		
		pub Kitties get(fn kitties): map hasher(blake2_128_concat) <T as Trait>::KittyIndex => Option<Kitty>;
		pub KittiesCount get(fn kitties_count): <T as Trait>::KittyIndex;
		pub KittyOwners get(fn kitties_owner):  map hasher(blake2_128_concat)  <T as Trait>::KittyIndex => Option<T::AccountId>;

		//map: AccountId-> [KittyIndex1, KittyIndex2 ...]
		pub KittyTotal get(fn kitty_total) : map hasher(blake2_128_concat) T::AccountId => vec::Vec<T::KittyIndex>;

		//map: KittyIndex-> (Parent1,Parent2)
        pub KittiesParents get(fn kitty_parents) : map hasher(blake2_128_concat) T::KittyIndex => (T::KittyIndex, T::KittyIndex);

        //map: KittyIndex-> [Children1, Children2 ...]
        pub KittiesChildren get(fn kitty_children): double_map hasher(blake2_128_concat) T::KittyIndex,  hasher(blake2_128_concat) T::KittyIndex => vec::Vec<T::KittyIndex>;

		//map: KittyIndex-> [Sibling1, Sibling2 ...]
        pub KittiesSibling get(fn kitty_sibling): map hasher(blake2_128_concat) T::KittyIndex => vec::Vec<T::KittyIndex>;

        //map: KittyIndex-> [Partner1, Partner2 ...]
        pub KittiesPartner get(fn kitty_partner) : map hasher(blake2_128_concat) T::KittyIndex => vec::Vec<T::KittyIndex>;
	}
}

// Pallets use events to inform users when important changes are made.
// https://substrate.dev/docs/en/knowledgebase/runtime/events
decl_event!(
	pub enum Event<T> where  AccountId = <T as frame_system::Trait>::AccountId, <T as Trait>::KittyIndex {
		Created(AccountId, KittyIndex),
		Transferred(AccountId, AccountId, KittyIndex),
		Breed(AccountId, KittyIndex, KittyIndex, KittyIndex),
	}
);

// Errors inform users that something went wrong.
decl_error! {
	pub enum Error for Module<T: Trait> {
		KittiesCountOverflow,
		InvalidKittyId,
		RequireDifferentParent,
		KittyNotExists,
        NotKittyOwner,
        TransferToSelf,
		MoneyNotEnough,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Errors must be initialized if they are used by the pallet.
		type Error = Error<T>;

		// Events must be initialized if they are used by the pallet.
		fn deposit_event() = default;

		#[weight = 0]
		pub fn create(origin) -> dispatch::DispatchResult {
			let sender = ensure_signed(origin)?;
			let kitty_id = Self::next_kitty_id()?;
			let dna = Self::random_value(&sender);
			let kitty = Kitty(dna);
			
			T::Currency::reserve(&sender, T::NewKittyReserve::get()).map_err(|_| Error::<T>::MoneyNotEnough)?;
			
			// 保存
			Self::insert_kitty(&sender, kitty_id, kitty);

			Self::deposit_event(RawEvent::Created(sender, kitty_id));
			Ok(())
		}

		#[weight = 0]
		pub fn transfer(origin, to: T::AccountId, kitty_id: T::KittyIndex) -> dispatch::DispatchResult {
			let sender = ensure_signed(origin)?;
			
			let owner = Self::kitties_owner(kitty_id).ok_or(Error::<T>::KittyNotExists)?;
			ensure!(owner == sender, Error::<T>::NotKittyOwner);
			ensure!(to != sender, Error::<T>::TransferToSelf);

			KittyOwners::<T>::insert(kitty_id, to.clone());

			KittyTotal::<T>::mutate(&sender, |val| val.retain(|&temp| temp == kitty_id));
			KittyTotal::<T>::mutate(&to, |val| val.push(kitty_id));
			
			Self::deposit_event(RawEvent::Transferred(sender, to, kitty_id));
			Ok(())
		}

		#[weight = 0]
		pub fn breed(origin, kitty_id_1: T::KittyIndex, kitty_id_2: T::KittyIndex) -> dispatch::DispatchResult {
			let sender = ensure_signed(origin)?;
			let new_kitty_id = Self::do_breed(&sender, kitty_id_1, kitty_id_2)?;
			Self::deposit_event(RawEvent::Breed(sender, kitty_id_1, kitty_id_2, new_kitty_id));
			Ok(())
		}
	}
}


impl <T: Trait>Module<T> {
	fn next_kitty_id() -> sp_std::result::Result<T::KittyIndex, DispatchError> {
		let kitty_id = Self::kitties_count(); // jusr get value
		if kitty_id == T::KittyIndex::max_value(){
			return Err(Error::<T>::KittiesCountOverflow.into())
		}

		Ok(kitty_id)
	}

	fn random_value(sender: &T::AccountId) -> [u8;16]{
		let payload = (
			T::Randomness::random_seed(),
			&sender,
			frame_system::Module::<T>::extrinsic_index(),
		);
		payload.using_encoded(blake2_128)
	}

	fn insert_kitty(owner: &T::AccountId, kitty_id: T::KittyIndex, kitty: Kitty){
		
		Kitties::<T>::insert(kitty_id, kitty);
		KittiesCount::<T>::put(kitty_id + 1.into());
		KittyOwners::<T>::insert(kitty_id, owner);


		if KittyTotal::<T>::contains_key(&owner){
			KittyTotal::<T>::mutate(owner, |val| val.push(kitty_id));
		}else{
			KittyTotal::<T>::insert(owner, vec![kitty_id]);
		}
	}

	// 记录父母
	fn update_kitties_parents(
        children: T::KittyIndex,
        father: T::KittyIndex,
        mother: T::KittyIndex,
    ) {
        <KittiesParents<T>>::insert(children, (father, mother));
    }

	// 记录孩子
    fn update_kitties_children(
        children: T::KittyIndex,
        father: T::KittyIndex,
        mother: T::KittyIndex,
    ) {
        if <KittiesChildren<T>>::contains_key(father, mother) {
            let _ = <KittiesChildren<T>>::mutate(father, mother, |val| val.push(children));
        } else {
            <KittiesChildren<T>>::insert(father, mother, vec![children]);
        }
	}
	
	// 记录兄弟姐妹
    fn update_kitties_sibling(kitty_id: T::KittyIndex) {
		let (father, mother) = KittiesParents::<T>::get(kitty_id);
		
		// 如果父母之前生过兄弟姐妹
        if <KittiesChildren<T>>::contains_key(father, mother) {
            let val: vec::Vec<T::KittyIndex> = KittiesChildren::<T>::get(father, mother);
            let reserve_val: vec::Vec<T::KittyIndex> =
                val.into_iter().filter(|&val| val != kitty_id).collect();
            <KittiesSibling<T>>::insert(kitty_id, reserve_val);
        } else {
			// 如果没有兄弟姐妹
            <KittiesSibling<T>>::insert(kitty_id, vec::Vec::<T::KittyIndex>::new());
        }
    }

    fn update_kitties_partner(partner1: T::KittyIndex, partner2: T::KittyIndex) {
		if KittiesPartner::<T>::contains_key(&partner1){
			let val: vec::Vec<T::KittyIndex> = KittiesPartner::<T>::get(partner1);
			let reserve_val: vec::Vec<T::KittyIndex> =
				val.into_iter().filter(|&val| val == partner2).collect();
			if reserve_val.len() == 0 {
				KittiesPartner::<T>::mutate(&partner1, |val| val.push(partner2));
			}
		}else{
			KittiesPartner::<T>::insert(partner1, vec![partner2]);
		};
    }


	fn combine_dna(dna1: u8, dna2: u8, selector: u8) -> u8{
		(selector & dna1) | (!selector & dna2)
	}

	fn do_breed(sender: &T::AccountId, kitty_id_1: T::KittyIndex, kitty_id_2: T::KittyIndex) -> sp_std::result::Result<T::KittyIndex, DispatchError> {
		
		// 不能是同一个kitty
		ensure!(kitty_id_1 != kitty_id_2, Error::<T>::RequireDifferentParent);

		T::Currency::reserve(&sender, T::NewKittyReserve::get()).map_err(|_| Error::<T>::MoneyNotEnough)?;

		
		// 是否存在
		let kitty1 = Self::kitties(kitty_id_1).ok_or(Error::<T>::InvalidKittyId)?;
		let kitty2 = Self::kitties(kitty_id_2).ok_or(Error::<T>::InvalidKittyId)?;

		let owner1 = Self::kitties_owner(kitty_id_1).ok_or(Error::<T>::KittyNotExists)?;
		let owner2 = Self::kitties_owner(kitty_id_2).ok_or(Error::<T>::KittyNotExists)?;
		
		// 判断KittyIndex是否属于发送者
		ensure!(owner1 == *sender, Error::<T>::NotKittyOwner);
		ensure!(owner2 == *sender, Error::<T>::NotKittyOwner);
		
		let kitty_id = Self::next_kitty_id()?;

		let kitty1_dna = kitty1.0;
		let kitty2_dna = kitty2.0;

		let selector = Self::random_value(&sender);
		let mut new_dna = [0u8; 16];

		for i in 0..kitty1_dna.len() {
			new_dna[i] = Self::combine_dna(kitty1_dna[i], kitty2_dna[i], selector[i]);
		}

		// 保存
		Self::insert_kitty(sender, kitty_id, Kitty(new_dna));

		// 更新配偶
        Self::update_kitties_partner(kitty_id_1, kitty_id_2);
        Self::update_kitties_partner(kitty_id_2,kitty_id_1);

        // 记录父母
        Self::update_kitties_parents(kitty_id, kitty_id_1, kitty_id_2);

        // 记录孩子
        Self::update_kitties_children(kitty_id, kitty_id_1, kitty_id_2);

       // 记录兄弟姐妹
        Self::update_kitties_sibling(kitty_id);


		Ok(kitty_id)
	}
}