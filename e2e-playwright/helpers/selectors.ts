/**
 * Centralised data-testid map. Prefer getByTestId(TID.xxx) over class/text selectors everywhere —
 * the share popup and question popup both render `.share-popup`, so class selectors are ambiguous.
 *
 * Every value here MUST correspond to a `data-testid` present in the Yew views.
 */
export const TID = {
  // Home
  home: 'home',
  homeCreateEvent: 'home-create-event',

  // New event
  neweventName: 'newevent-name',
  neweventEmail: 'newevent-email',
  neweventDesc: 'newevent-desc',
  neweventFinish: 'newevent-finish',

  // Event page — load state + shell
  eventLoadstate: 'event-loadstate', // + data-state = loading | notfound | deleted | loaded
  eventLoaded: 'event-loaded',
  eventName: 'event-name',
  eventDesc: 'event-desc',

  // Ask a question
  askButton: 'ask-button',
  askButtonTopbar: 'ask-button-topbar',
  questionInput: 'question-input',
  questionSubmit: 'question-submit',

  // Questions
  questionItem: 'question-item', // + data-qid = question id
  questionLike: 'question-like',
  questionLikeCount: 'question-like-count',
  questionHide: 'question-hide',
  questionAnswer: 'question-answer',
  questionsBucket: 'questions-bucket', // + data-bucket

  // Share popup
  shareLinkbox: 'share-linkbox',
  shareCopy: 'share-copy',

  // Password popup
  passwordPopup: 'password-popup',
  passwordInput: 'password-input',

  // Connection / offline
  offlineIndicator: 'offline-indicator',
  topbar: 'topbar', // + data-connected = true | false

  // Moderator controls
  modStateSelect: 'mod-state-select',
  modDelete: 'mod-delete',
} as const;

export type TestId = (typeof TID)[keyof typeof TID];

/** data-state values on TID.eventLoadstate. */
export const LOAD_STATE = {
  loading: 'loading',
  notfound: 'notfound',
  deleted: 'deleted',
  loaded: 'loaded',
} as const;
