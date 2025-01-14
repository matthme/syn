import { Base64 } from 'js-base64';
import path from 'path';
import { fileURLToPath } from 'url';
import { TextEditorDelta, textEditorGrammar, TextEditorState } from './grammar';
import { SynGrammar } from '@holochain-syn/store';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

export const synDna = path.join(__dirname, '../../dna/workdir/dna/syn.dna');

export type Add = [number, string];
export type Delete = [number, number];
export type Title = string;

// Signal type definitions
export type Delta = {
  type: string;
  value: Add | Delete | Title;
};

export type Signal = {
  sessionHash: string;
  message: {
    type: string;
    payload: any;
  };
};

export const delay = ms => new Promise(r => setTimeout(r, ms));

export function serializeHash(hash: Uint8Array): string {
  return `u${Base64.fromUint8Array(hash, true)}`;
}

/*
  Fake UI functions
    - applyDeltas
      - takes a content and a list of deltas
      - returns the new content with those deltas applied
*/

export interface Content {
  title: string;
  body: TextEditorState;
}

export type TextDelta =
  | {
      type: 'Title';
      value: string;
    }
  | TextEditorDelta;

export const applyDelta = (
  content: Content,
  delta: TextDelta,
  author: string
): Content => {
  switch (delta.type) {
    case 'Title':
      content.title = delta.value;
      return content;
    default:
      return {
        ...content,
        body: textEditorGrammar.applyDelta(content.body, delta, author),
      };
  }
};

export const sampleGrammar: SynGrammar<Content, TextDelta> = {
  initialState: {
    title: '',
    body: textEditorGrammar.initialState,
  },
  applyDelta,
  transformDelta(toTransform, withOperation) {
    if (toTransform.type === 'Title' || withOperation.type == 'Title')
      return toTransform;
    return (textEditorGrammar.transformDelta as any)(
      toTransform,
      withOperation
    );
  },
};

export function applyDeltas(
  content: Content,
  deltas: TextDelta[],
  author: string
): Content {
  for (const delta of deltas) {
    content = applyDelta(content, delta, author);
  }
  return content;
}
