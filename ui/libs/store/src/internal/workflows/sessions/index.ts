import type { EntryHashB64 } from '@holochain-open-dev/core-types';
import { get } from 'svelte/store';
import type { SynGrammar } from '../../../grammar';
import { amIScribe } from '../../../state/selectors';
import type { SynWorkspace } from '../../workspace';
import { folkLeaveSession } from './folk';
import { closeSession, CloseSessionResult } from './scribe';

export async function leaveSession<G extends SynGrammar<any, any>>(
  workspace: SynWorkspace<G>,
  sessionHash: EntryHashB64
): Promise<CloseSessionResult | undefined> {
  let state = get(workspace.store);

  if (amIScribe(state, sessionHash)) {
    return closeSession(workspace, sessionHash);
  } else {
    return folkLeaveSession(workspace, sessionHash) as Promise<
      CloseSessionResult | undefined
    >;
  }
}
