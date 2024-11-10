use ic_cdk::api::{caller, is_controller};
use ic_principal::Principal;
use ic_stable_structures::memory_manager::{MemoryId, VirtualMemory};
use ic_stable_structures::GrowFailed;
use ic_stable_structures::{
    memory_manager::MemoryManager as MM, storable::Bound, DefaultMemoryImpl as DefMem,
    StableBTreeMap, Storable,
};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Default)]
pub struct Cbor<T>(pub T)
where
    T: serde::Serialize + serde::de::DeserializeOwned;

impl<T> std::ops::Deref for Cbor<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Storable for Cbor<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    fn to_bytes(&self) -> Cow<[u8]> {
        let mut buf = vec![];
        ciborium::ser::into_writer(&self.0, &mut buf).unwrap();
        Cow::Owned(buf)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Self(ciborium::de::from_reader(bytes.as_ref()).unwrap())
    }

    const BOUND: Bound = Bound::Unbounded;
}

const CONTENT_MEM_ID: MemoryId = MemoryId::new(0);
const USERS_MEM_ID: MemoryId = MemoryId::new(1);
const CONTENT_INDEX_MEM_ID: MemoryId = MemoryId::new(2);

type VM = VirtualMemory<DefMem>;

pub type ContentPath = String;

pub type Content = String;

pub type UserId = Principal;

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub enum UserAccess {
    None,
    Read,
    ReadWrite,
}

thread_local! {
  static MEMORY_MANAGER: RefCell<MM<DefMem>> =
    RefCell::new(MM::init(DefMem::default()));
  pub static CONTENT_STORE: RefCell<StableBTreeMap<ContentPath, Content, VM>> =
    MEMORY_MANAGER.with(|mm| {
      RefCell::new(StableBTreeMap::init(mm.borrow().get(CONTENT_MEM_ID)))
    });
  pub static USERS_STORE: RefCell<StableBTreeMap<UserId, Cbor<HashMap<ContentPath, UserAccess>>, VM>> =
    MEMORY_MANAGER.with(|mm| {
      RefCell::new(StableBTreeMap::init(mm.borrow().get(USERS_MEM_ID)))
    });
  pub static CONTENT_INDEX: RefCell<StableBTreeMap<u64, ContentPath, VM>> = MEMORY_MANAGER.with(|mm| {
    RefCell::new(StableBTreeMap::init(mm.borrow().get(CONTENT_INDEX_MEM_ID)))
  });
}

fn content_index_flush() {
    CONTENT_INDEX.with(|ci| {
        ci.borrow_mut().clear_new();
    })
}

fn content_index_populate(index: Vec<ContentPath>) -> Result<(), GrowFailed> {
    CONTENT_INDEX.with(|ci| {
        let mut ci = ci.borrow_mut();
        let mut i = 0;
        for cp in index {
            ci.insert(i, cp);
            i += 1;
        }
        Ok(())
    })
}

pub fn content_index_set(index: &Vec<ContentPath>) -> Result<(), GrowFailed> {
    content_index_flush();
    content_index_populate(index.clone())
}

pub fn content_index_lookup(index: u64) -> Result<ContentPath, u64> {
    CONTENT_INDEX.with(|ci| ci.borrow().get(&index).ok_or_else(|| ci.borrow().len() - 1))
}

pub fn user_access(path: &ContentPath, user_id: UserId) -> UserAccess {
    if is_controller(&caller()) {
        return UserAccess::ReadWrite;
    }
    let users_acl = USERS_STORE
        .with(|us| us.borrow().get(&user_id))
        .map(|u| u.to_owned().clone())
        .unwrap_or_default();
    users_acl.get(path).unwrap_or(&UserAccess::None).clone()
}

pub fn user_give_access(path: &ContentPath, user_id: UserId) {
    if is_controller(&caller()) {
        return;
    }
    USERS_STORE.with(|us| {
        let mut acl = us.borrow().get(&user_id).unwrap_or_default().clone();
        acl.insert(path.to_string(), UserAccess::Read);
        us.borrow_mut().insert(user_id, Cbor(acl))
    });
}

pub fn user_access_list(user_id: UserId) -> Vec<ContentPath> {
    let acl: HashMap<String, UserAccess> =
        USERS_STORE.with(|us| us.borrow().get(&user_id).unwrap_or_default().clone());
    acl.into_iter()
        .filter_map(|(k, a)| match a {
            UserAccess::None => None,
            _ => Some(k.clone()),
        })
        .collect()
}

pub fn page_get(path: ContentPath) -> Option<Content> {
    CONTENT_STORE.with(|cs| cs.borrow().get(&path))
}

pub fn pages_get() -> Option<HashMap<ContentPath, Content>> {
    if is_controller(&caller()) {
        let mut pages = HashMap::new();
        CONTENT_STORE.with(|cs| {
            for (path, page) in cs.borrow().iter() {
                pages.insert(path, page);
            }
        });
        Some(pages)
    } else {
        None
    }
}

pub fn page_set(path: ContentPath, content: Content) -> Option<Content> {
    CONTENT_STORE.with(|cs| cs.borrow_mut().insert(path, content))
}

pub fn page_delete(path: ContentPath) -> Option<Content> {
    CONTENT_STORE.with(|cs| cs.borrow_mut().remove(&path))
}
