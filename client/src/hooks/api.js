import { PublicClientApplication } from "@azure/msal-browser";

const API_BASE_URL =
  import.meta.env.MODE === "development"
    ? "/api"
    : import.meta.env.VITE_API_BASE_URL || "/api";

const msalConfig = {
  auth: {
    clientId: import.meta.env.VITE_CLIENT_ID,
    authority: import.meta.env.VITE_AUTHORITY,
    redirectUri: window.location.origin,
  },
};

const pca = new PublicClientApplication(msalConfig);

async function logoutAndClearStorage() {
  localStorage.removeItem("msalAccount");
  localStorage.removeItem("msalAccessToken");
  await pca.logout();
}

async function refreshAccessToken() {
  const account = JSON.parse(localStorage.getItem("msalAccount"));

  if (account) {
    const silentTokenRequest = {
      scopes: ["openid", "profile", "User.Read"],
      account,
    };

    try {
      const silentTokenResponse = await pca.acquireTokenSilent(
        silentTokenRequest
      );
      localStorage.setItem("msalAccessToken", silentTokenResponse.accessToken);
      return silentTokenResponse.accessToken;
    } catch (error) {
      console.error("Token refresh failed:", error);
      return null;
    }
  }

  return null;
}

export async function fetchData(endpoint, options = {}) {
  const defaultHeaders = {
    Authorization: `Bearer ${localStorage.getItem("msalAccessToken")}`,
  };

  const requestOptions = {
    method: "GET",
    headers: {
      ...defaultHeaders,
      ...options.headers,
    },
    ...options,
  };

  const response = await fetch(`${API_BASE_URL}${endpoint}`, requestOptions);

  if (response.status === 401) {
    accessToken = await refreshAccessToken();

    if (accessToken) {
      requestOptions.headers.Authorization = `Bearer ${accessToken}`;
      response = await fetch(`${API_BASE_URL}${endpoint}`, requestOptions);
    } else {
      await logoutAndClearStorage();
      throw new Error("Unauthorized: Access token is invalid or expired.");
    }
  }

  const data = await response.json();
  return data;
}
