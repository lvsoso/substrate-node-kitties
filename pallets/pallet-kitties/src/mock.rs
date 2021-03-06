use crate::{Module, Trait};
use sp_core::H256;
use frame_support::{
	traits::{OnFinalize, OnInitialize},
	impl_outer_event, impl_outer_origin, parameter_types, weights::Weight};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup}, testing::Header, Perbill,
};
use frame_system as system;

use balances;

impl_outer_origin! {
	pub enum Origin for TestRuntime {}
}

// Configure a mock runtime to test the pallet.

#[derive(Clone, Eq, PartialEq)]
pub struct TestRuntime;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);

	pub const ExistentialDeposit: u64 = 1;
    pub const TransferFee: u64 = 1;
    pub const CreationFee: u64 = 1;
}

impl system::Trait for TestRuntime {
	type BaseCallFilter = ();
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = TestEvent;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type PalletInfo = ();
	type AccountData = balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
}

mod kitties_event {
    pub use crate::Event;
}

impl_outer_event! {
    pub enum TestEvent for TestRuntime {
        system<T>, //??
        kitties_event<T>, //??
        balances<T>, //??
    }
}

impl balances::Trait for TestRuntime {
    type Balance = u64;
    type MaxLocks = ();
    type Event = TestEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = system::Module<TestRuntime>; 
    type WeightInfo = ();
}

type Randomness = pallet_randomness_collective_flip::Module<TestRuntime>;

parameter_types! {
    pub const NewKittyReserve: u64 = 5_000;
}

impl Trait for TestRuntime {
	type Event = TestEvent;
	type Randomness = Randomness;
	type KittyIndex = u32;

	type NewKittyReserve = NewKittyReserve;
    type Currency = balances::Module<Self>;
}

pub type KittiesModule = Module<TestRuntime>;
pub type System = frame_system::Module<TestRuntime>;

pub fn run_to_block(n: u64) {
    while System::block_number() < n {
        KittiesModule::on_finalize(System::block_number());
        System::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        KittiesModule::on_initialize(System::block_number());
    }
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::default()
        .build_storage::<TestRuntime>()
        .unwrap()
        .into();

    balances::GenesisConfig::<TestRuntime> {
        balances: vec![(1, 5000000), (2, 51000000), (3, 5200000), (4, 53000000), (5, 54000000)],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
