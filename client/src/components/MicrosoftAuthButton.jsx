import { PublicClientApplication } from "@azure/msal-browser";
import React, { useCallback, useMemo } from "react";

const MicrosoftAuthButton = ({ clientId, authority, scopes }) => {
  const msalConfig = {
    auth: {
      clientId,
      authority,
      redirectUri: window.location.origin,
    },
  };

  const pca = useMemo(
    () => new PublicClientApplication(msalConfig),
    [msalConfig]
  );

  const handleLogin = useCallback(async () => {
    try {
      const loginRequest = {
        scopes,
      };

      const loginResponse = await pca.loginPopup(loginRequest);
      localStorage.setItem(
        "msalAccount",
        JSON.stringify(loginResponse.account)
      );
      localStorage.setItem("msalAccessToken", loginResponse.accessToken);

      window.location.reload();
    } catch (error) {
      console.error("Login failed:", error);
    }
  }, [pca, scopes]);

  return <button onClick={handleLogin}>Sign in with Microsoft</button>;
};

export default MicrosoftAuthButton;
