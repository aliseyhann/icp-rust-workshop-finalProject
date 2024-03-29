//import
use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpMethod,
};

use candid:: {CandidType, Decode, Deserialize, Encode};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::env::consts;
use std::{borrow::Cow, cell::RefCell};

#[derive(CandidType, Deserialize, Clone)]
// struct olusturma
struct Participant {
    address: String,
}

// event belirleme, struct
#[derive(CandidType, Deserialize, Clone)]
struct Event {
    name: String,
    date: String,
    #[serde(default)] // Vektörün içini boşaltmak için kullanılan yapı
    participants: Vec<Participant>, 
}

#[derive(CandidType, Deserialize)]
enum EventError {
    NoSuchEvent,
    JoinError,
    CancelJoinError,
    GetEventsError,
    AlreadyJoined,
    AlreadyExists
}

//implementation
impl Storable for Event {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

type Memory = VirtualMemory<DefaultMemoryImpl>;
const MAX_VALUE_SIZE: u32 = 100;

//implement BoundedStorable for Event
impl BoundedStorable for Event {
    const MAX_SIZE: u32 = MAX_VALUE_SIZE; // katılımcı sayısını sabitleme, ayarlama
    const IS_FIXED_SIZE: bool = false;
}

//yeni MemoryId -> thread_local!
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
    RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static EVENTS_MAP: RefCell<StableBTreeMap<u64, Event, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1))), // farkli bir memoryId
        )
    );
}

// bir etkinlik yaratıp, depola
#[ic_cdk::update]
fn create_event(name: String, date: String) -> Result<(), EventError> {
    EVENTS_MAP.with(|events_map_ref| {
        let mut events_map = events_map_ref.borrow_mut();

        //böyle bir etkinlik ismi ve tarihi var mı yok mu?
        for(_, event) in events_map.iter() {
            if event.name == name && event.date == date {
                Err(EventError::AlreadyExists)
            }
        }

        // eger bi etkinlik yoksa, yeni bir tane olustur
        let new_event = Event {
            name,
            date,
            participants: Vec::new(),
        };

        let new_event_id = events_map.len();
        events_map.insert(new_event_id, new_event);

        Ok(())

    })
}

// etkinligin katılımcıları
#[ic_cdk::update]
fn join_event(event_id: u64, participant_address: String) -> Result<(), EventError> {
    EVENTS_MAP.with(|events_map_ref| {
        let mut events_map = events_map_ref.borrow_mut();

        // evetn aldık, klonladık, degistircez, update etcez.
        if let Some(mut event) = events_map.get(&event_id) {
            if event.participants.iter().any(|p| p.address == participant_address) {
                return Err(EventError::AlreadyJoined);
            }

            let new_participant = Participant {address: participant_address};
            event.participants.push(new_participant);
           
            events_map.insert(event_id, event);
            Ok(())
        } else {
            Err(EventError::NoSuchEvent)
        }
    })
}

// katılımcının katılmayı düsündügü etkinlie gitmemesi
#[ic_cdk::update]
fn cancel_join_event(event_id: u64, participant_address: String) -> Result<(), EventError> {
    EVENTS_MAP.with(|events_map_ref| {
        let mut events_map = events_map_ref.borrow_mut();

        // event aldık, klonladık, degistircez, update etcez.
        if let Some(mut event) = events_map.get(&event_id) {
            if let Some(index) = event.participants.iter().position(|p| p.address == participant_address) {
                event.participants.remove(index);
                events_map.insert(event_id, event);
                Ok(());
            } else {
                Err(EventError::CancelJoinError);
            }
        }
        Err(EventError::NoSuchEvent)
    })
}
