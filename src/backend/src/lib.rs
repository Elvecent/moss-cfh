use candid::{CandidType, Deserialize};
use ic_cdk::{query, update};
use lazy_static::lazy_static;
use std::collections::HashMap;

mod store;
use store::{
    content_index_set, page_delete, page_set, pages_get, user_access, user_access_list,
    user_give_access, Content, ContentPath, UserId,
};

use crate::store::{content_index_lookup, page_get, UserAccess};

lazy_static! {
    static ref PAGES: HashMap<&'static str, &'static str> = HashMap::from([
        (
            "index.html",
            r#"
<!DOCTYPE html>
<html lang="en">

<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width" />
  <title>Page 1</title>
  <base href="/" />
</head>

<body>
  <h2>Hello from page1</h2>
</body>

</html>
    "#
        ),
        (
            "stuff.html",
            r#"
<!DOCTYPE html>
<html lang="en">

<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width" />
  <title>Page 2</title>
  <base href="/" />
</head>

<body>
  <h2>Hello from page2</h2>
</body>

</html>
    "#
        )
    ]);
}

#[derive(CandidType)]
enum GetPageRes {
    Page { path: ContentPath, content: Content },
    FundingRequired { path: ContentPath, user_id: UserId },
    PathNotFound { path: ContentPath },
    IndexOutOfBounds { index: u64, max_index: u64 },
}

#[derive(CandidType, Deserialize)]
struct GetPageArgs {
    index: u64,
}

#[query]
fn get_page(GetPageArgs { index }: GetPageArgs) -> GetPageRes {
    use GetPageRes::*;
    let user_id = ic_cdk::caller();
    let path = match content_index_lookup(index.clone()) {
        Ok(p) => p,
        Err(n) => {
            return IndexOutOfBounds {
                index,
                max_index: n,
            }
        }
    };
    match user_access(&path, user_id.clone()) {
        UserAccess::None => FundingRequired { path, user_id },
        _ => page_get(path.clone())
            .map(|p| Page {
                content: p,
                path: path.clone(),
            })
            .unwrap_or(PathNotFound { path }),
    }
}

#[derive(CandidType, Deserialize)]
struct SetPageArgs {
    path: ContentPath,
    content: Content,
}

#[derive(CandidType, Deserialize)]
enum SetPageRes {
    Set { path: ContentPath },
    AccessDenied { path: ContentPath, user_id: UserId },
}

#[update]
fn set_page(SetPageArgs { path, content }: SetPageArgs) -> SetPageRes {
    let user_id = ic_cdk::api::caller();
    use SetPageRes::*;
    match user_access(&path, user_id) {
        UserAccess::ReadWrite => {
            page_set(path.clone(), content);
            Set { path }
        }
        _ => AccessDenied { path, user_id },
    }
}

#[derive(CandidType, Deserialize)]
struct DeletePageArgs {
    path: ContentPath,
}

#[derive(CandidType, Deserialize)]
enum DeletePageRes {
    Deleted { path: ContentPath },
    NotFound { path: ContentPath },
    AccessDenied { path: ContentPath, user_id: UserId },
}

#[update]
fn delete_page(DeletePageArgs { path }: DeletePageArgs) -> DeletePageRes {
    let user_id = ic_cdk::api::caller();
    use DeletePageRes::*;
    match user_access(&path, user_id) {
        UserAccess::ReadWrite => page_delete(path.clone())
            .map(|_| Deleted { path: path.clone() })
            .unwrap_or(NotFound { path }),
        _ => AccessDenied { path, user_id },
    }
}

#[derive(CandidType, Deserialize)]
struct FundPageArgs {
    path: ContentPath,
}

#[derive(CandidType, Deserialize)]
enum FundPageRes {
    Funded {
        spent_amount: f32, // approximate
        pages: Vec<ContentPath>,
    },
    InsufficientFunds,
}

#[update]
fn fund_page(FundPageArgs { path }: FundPageArgs) -> FundPageRes {
    let user_id = ic_cdk::api::caller();
    use FundPageRes::*;
    // TODO: spend user's balance
    user_give_access(&path, user_id);
    Funded {
        spent_amount: 0.0, // TODO
        pages: vec![path],
    }
}

#[query]
fn funded_pages_list() -> Vec<ContentPath> {
    let user_id = ic_cdk::api::caller();
    user_access_list(user_id)
}

#[derive(CandidType, Deserialize)]
enum AllPagesRes {
    Pages(HashMap<ContentPath, Content>),
    AccessDenied { user_id: UserId },
}

#[query]
fn all_pages() -> AllPagesRes {
    use AllPagesRes::*;
    match pages_get() {
        Some(ps) => Pages(ps),
        None => AccessDenied {
            user_id: ic_cdk::api::caller(),
        },
    }
}

#[derive(CandidType, Deserialize)]
enum CheckBalanceRes {
    What,
}

#[query]
fn check_balance() -> CheckBalanceRes {
    use CheckBalanceRes::*;
    let user_id = ic_cdk::api::caller();
    // TODO: call ledger canister to see caller's actual balance
    What
}

#[derive(CandidType, Deserialize)]
struct SetIndexArgs {
    index: Vec<ContentPath>,
}

#[derive(CandidType, Deserialize)]
enum SetIndexRes {
    IndexSet(Vec<ContentPath>),
    StorageFailure,
}

#[update]
fn set_index(SetIndexArgs { index }: SetIndexArgs) -> SetIndexRes {
    use SetIndexRes::*;
    match content_index_set(&index) {
        Ok(_) => IndexSet(index),
        _ => StorageFailure,
    }
}

#[query]
fn whoami() -> ic_principal::Principal {
    ic_cdk::api::caller()
}

ic_cdk::export_candid!();
