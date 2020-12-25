use error::{SynError, SynResult};
use hdk3::prelude::*;
use link::LinkTag;
mod error;

entry_defs![
    Path::entry_def(),
    Content::entry_def(),
    ContentChange::entry_def(),
    Session::entry_def()
];


/// Content
// This is structure holds the shared content that is being collaboratively
// edited by participants in the happ
#[hdk_entry(id = "content")]
#[derive(Clone, Debug, Default)]
pub struct Content {
    pub title: String,
    pub body: String,
}

/// Session
/// This entry holds the record of who the scribe is and a hash
/// of the content at the start of the session
/// the scribe will always be the author of the session
#[hdk_entry(id = "session")]
#[derive(Debug)]
struct Session {
    pub snapshot: HeaderHash,  // hash of the starting state for this session
    // scribe:  // scribe will always be the author of the session, look in the header
}

fn put_content_inner(content: Content) -> SynResult<(HeaderHash, EntryHash)> {
    let header_hash = create_entry(&content)?;
    let content_hash = hash_entry(&content)?;

    let path = get_snapshots_path();
    path.ensure()?;

    // snapshot anchor base
    let snapshots_anchor_hash = path.hash()?;
    create_link(
        snapshots_anchor_hash,
        content_hash.clone(),
        (),
    )?;
    Ok((header_hash, content_hash))
}

// Used by the clerk to commit a snapshot of the content and link it to
// the snapshot anchor.
#[hdk_extern]
pub fn put_content(content: Content) -> SynResult<EntryHash> {
    let (_, content_hash) = put_content_inner(content)?;
    Ok(content_hash)
}

/// The optional content
#[derive(Serialize, Deserialize, SerializedBytes)]
pub struct OptionContent(Option<Content>);

#[hdk_extern]
fn get_content(input: EntryHash) -> SynResult<OptionContent> {
    if let Some(element) = get(input,  GetOptions::content())? {
        Ok(OptionContent(element.into_inner().1.to_app_option()?))
    } else {
        Ok(OptionContent(None))
    }
}

/// Delta
/// change this for your app to indicate a small change in a patch-grammar
/// appropriate for your content.  In this example the grammar is an indicator
/// to delete or add text at a given offset in the body, or to set a title value
#[derive(Clone, Serialize, Deserialize, SerializedBytes, Debug)]
#[serde(tag = "type", content = "content")]
pub enum Delta {
    Title(String),
    Add((usize,String)),
    Delete((usize,usize)),
}

///  Content Change
#[derive(Clone, Serialize, Deserialize, SerializedBytes, Debug)]
pub struct ChangeMeta {
    pub contributors: Vec<AgentPubKey>,
    pub witnesses: Vec<AgentPubKey>, // maybe?
    pub app_specific: Option<SerializedBytes>,
}

/// Entry type for committing changes to the content, called by the clerk.
#[hdk_entry(id = "content_change")]
#[derive(Clone, Debug)]
pub struct ContentChange {
    pub deltas: Vec<Delta>,
    pub previous_change: EntryHash, // hash of Content on which these deltas are to be applied
    pub meta: ChangeMeta,
}

/// Input to the commit call
#[derive(Serialize, Deserialize, SerializedBytes, Debug)]
pub struct CommitInput {
    pub snapshot: EntryHash,
    pub change: ContentChange,
}

#[hdk_extern]
fn commit(input: CommitInput) -> SynResult<HeaderHash> {
    let header_hash = create_entry(&input.change)?;
    let change_hash = hash_entry(&input.change)?;
    let bytes: SerializedBytes = input.change.previous_change.try_into()?;
    let tag = LinkTag::from(bytes.bytes().to_vec());
    create_link(
        input.snapshot,
        change_hash.clone(),
        tag,
    )?;
    Ok(header_hash)
}

#[hdk_extern]
fn hash_content(content: Content) -> SynResult<EntryHash> {
    let hash = hash_entry(&content)?;
    Ok(hash)
}

///  Session Info needed to start working in a session
#[derive(Clone, Serialize, Deserialize, SerializedBytes, Debug)]
pub struct SessionInfo {
    pub session: HeaderHash,
    pub scribe: AgentPubKey,
    pub content: Content,
    pub content_hash: EntryHash,
}

fn get_sessions_path() -> Path {
    Path::from("sessions")
}

fn get_snapshots_path() -> Path {
    Path::from("snapshots")
}

fn create_session(session: Session) -> SynResult<HeaderHash> {
    let path = get_sessions_path();
    path.ensure()?;
    let header_hash = create_entry(&session)?;
    let session_hash = hash_entry(&session)?;

    let session_anchor_hash = path.hash()?;
    create_link(
        session_anchor_hash,
        session_hash,
        (),
    )?;

    Ok(header_hash)
}

/// builds the content out from a given snapshot by looking at commits on that snapshot
/// return error if hash not found rather than option because we
/// shouldn't be calling this function on a hash that doesn't exist
fn build_content_from_snapshot(header_hash: HeaderHash) -> SynResult<(EntryHash, Content)> {
    if let Some(element) = get(header_hash, GetOptions::content())? {
        let maybe_content: Option<Content> = element.entry().to_app_option()?;
        if let Some(mut content) = maybe_content {
            // get commits from this snapshot
            // TODO: we should be able to get the entry hash from the
            // element, but I don't quite know how to yet
            let snapshot_hash = hash_entry(&content)?;
            let commits = get_links_and_load_type::<ContentChange>(snapshot_hash, None)?;
            // TODO sort commits in correct order
            for commit in commits {
                for delta in commit.deltas {
                    match delta {
                        Delta::Title(new) => content.title = new,
                        Delta::Add((start, value)) => content.body.insert_str(start, &value),
                        Delta::Delete((start, stop)) => content.body.replace_range(start..stop, ""),
                    }
                }
            }
            return Ok((hash_entry(&content)?, content));
        };
    };
    Err(SynError::HashNotFound)
}

/// builds out the session info from a given session hash.
/// return error if hash not found rather than option because we
/// shouldn't be calling this function on a hash that doesn't exist
fn build_session_info(session_hash: EntryHash) -> SynResult<SessionInfo> {
    if let Some(element) = get(session_hash,  GetOptions::content())? {
        let maybe_session: Option<Session> = element.entry().to_app_option()?;
        if let Some(session) = maybe_session {
            let (content_hash, content) = build_content_from_snapshot(session.snapshot)?;
            debug!("h {:?}, c {:?}", content_hash, content);
            return Ok(SessionInfo {
                scribe: element.header().author().clone(),
                session: element.header_address().clone(),
                content,
                content_hash
            });
        };
    };
    Err(SynError::HashNotFound)
}

#[hdk_extern]
fn join_session(_: ()) -> SynResult<SessionInfo> {

    // get recent sessions
    let sessions = get_sessions_inner()?;
    // see if there's an active one
    if sessions.len() > 0 {
        debug!("session found: {:?}", sessions);
        // for now just pick the first!
        Ok(build_session_info(sessions[0].clone())?)
    }
    else {
        debug!("no sessions found");

        // fall back to other users

        // can't find a session so make one ourself
        // 1. find the Content we will make our session off of
        // TODO
        // 2. can't find a Content assume null content and commit it
        let content = Content::default();
        let (header_hash, content_hash) = put_content_inner(content.clone())?;

        let scribe = agent_info()?.agent_latest_pubkey;
        let session = Session {
            snapshot: header_hash,
            // scribe is author
        };
        let session_hash = create_session(session)?;

        Ok(SessionInfo{
            scribe,
            session: session_hash,
            content,
            content_hash,
        })
    }
}

fn get_sessions_inner() -> SynResult<Vec<EntryHash>> {
    let path = get_sessions_path();
    let links = get_links(path.hash()?, None)?.into_inner();
    let sessions = links.into_iter().map(|l| l.target).collect();
    Ok(sessions)
}

#[derive(Clone, Serialize, Deserialize, SerializedBytes, Debug)]
pub struct SessionList(Vec<EntryHash>);
#[hdk_extern]
pub fn get_sessions(_: ()) -> SynResult<SessionList> {
    let sessions = get_sessions_inner()?;
    Ok(SessionList(sessions))
}


pub fn get_links_and_load_type<R: TryFrom<Entry>>(
    base: EntryHash,
    tag: Option<LinkTag>,
) -> SynResult<Vec<R>> {
    let links = get_links(base.into(), tag)?.into_inner();

    Ok(links
       .iter()
       .map(
           |link|  {
               if let Some(element) = get(link.target.clone(), Default::default())? {
                   let e: Entry = element.entry().clone().into_option().ok_or(SynError::HashNotFound)?;
                   let entry: R = R::try_from(e).map_err(|_e| SynError::HashNotFound)?;
                   return Ok(entry);
               };
               Err(SynError::HashNotFound)
           },
       )
       .filter_map(Result::ok)
       .collect())
}