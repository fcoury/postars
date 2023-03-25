import React, { createContext, useContext, useReducer } from "react";

const AppStateContext = createContext();

const initialState = {
  currentFolder: "Inbox",
  emails: [],
  email: null,
  loadingEmails: false,
  loadingEmail: false,
  isLoggedIn: false,
  currentEmailIndex: null,
  totalEmails: 0,
  emailsLoading: [],
};

function reducer(state, action) {
  const result = reducerx(state, action);
  console.log("result", result);
  return result;
}

function reducerx(state, action) {
  console.log("----");
  console.log("reducer", action);
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
      const currentIndex = state.email
        ? state.emails.findIndex((email) => email.id === state.email.id)
        : 0;

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

    case "addEmailLoading":
      return {
        ...state,
        emailsLoading: [...state.emailsLoading, action.payload],
      };

    case "removeEmailLoading":
      return {
        ...state,
        emailsLoading: state.emailsLoading.filter(
          (id) => id !== action.payload
        ),
      };

    case "deselectEmail":
      return { ...state, email: null, currentEmailIndex: null };

    case "setCurrentFolder":
      return { ...state, currentFolder: action.payload };

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
