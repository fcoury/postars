import React, { createContext, useContext, useReducer } from "react";

const AppStateContext = createContext();

const initialState = {
  emails: [],
  email: null,
  loadingEmails: false,
  loadingEmail: false,
  isLoggedIn: false,
  currentEmailIndex: null,
  totalEmails: 0,
};

function reducer(state, action) {
  switch (action.type) {
    case "setLoggedIn":
      return { ...state, isLoggedIn: action.payload };

    case "setLoggedOut":
      return { ...state, isLoggedIn: false };

    case "setLoadingEmails":
      return { ...state, loadingEmails: action.payload };

    case "setLoadingEmail":
      return { ...state, loadingEmail: action.payload };

    case "setEmails":
      return {
        ...state,
        emails: action.payload,
        totalEmails: action.payload.length,
      };

    case "clearEmails":
      return { ...state, emails: [] };

    case "removeEmail":
      return {
        ...state,
        emails: state.emails.filter((email) => email.id !== action.payload),
      };

    case "updateEmail":
      const updatedEmails = state.emails.map((email) =>
        email.id === action.payload.id
          ? { ...email, ...action.payload.updates }
          : email
      );
      return { ...state, emails: updatedEmails };

    case "setSelectedEmail":
      const selectedIndex = state.emails.findIndex(
        (email) => email.id === action.payload.id
      );
      return {
        ...state,
        email: action.payload,
        currentEmailIndex: selectedIndex,
      };

    case "nextEmail":
      const currentIndex = state.emails.findIndex(
        (email) => email.id === state.email.id
      );

      if (currentIndex < state.emails.length - 1) {
        return {
          ...state,
          email: state.emails[currentIndex + 1],
          currentEmailIndex: currentIndex + 1,
        };
      }
      return state;

    case "previousEmail":
      const prevIndex = state.emails.findIndex(
        (email) => email.id === state.email.id
      );

      if (prevIndex > 0) {
        return {
          ...state,
          email: state.emails[prevIndex - 1],
          currentEmailIndex: prevIndex - 1,
        };
      }
      return state;

    default:
      throw new Error();
  }
}

export function AppStateProvider({ children, initialEmails = [] }) {
  const [state, dispatch] = useReducer(reducer, {
    ...initialState,
    emails: initialEmails,
  });

  return (
    <AppStateContext.Provider value={{ state, dispatch }}>
      {children}
    </AppStateContext.Provider>
  );
}

export function useAppState() {
  const context = useContext(AppStateContext);

  if (context === undefined) {
    throw new Error("useAppState must be used within a AppStateProvider");
  }

  return context;
}
