import React, { useEffect, useRef } from "react";
import { useAppState } from "../../state/AppState";

function EmailBody() {
  const { state } = useAppState();
  const { email } = state;
  const iframeRef = useRef(null);

  useEffect(() => {
    if (email && iframeRef.current) {
      const iframeDoc = iframeRef.current.contentWindow.document;

      const styleElement = iframeDoc.createElement("link");

      styleElement.setAttribute("rel", "stylesheet");
      styleElement.setAttribute("type", "text/css");
      styleElement.setAttribute(
        "href",
        `${import.meta.env.BASE_URL}email-styles.css`
      );

      iframeDoc.open();
      iframeDoc.write(email.body.content);
      iframeDoc.head.appendChild(styleElement);
      iframeDoc.close();
    }
  }, [email]);

  if (!email) {
    return <div>Please select an email to view its content.</div>;
  }

  return (
    <div className="body">
      <iframe
        ref={iframeRef}
        title="email-content"
        frameBorder="0"
        sandbox="allow-same-origin allow-scripts"
        style={{ flexGrow: 1, width: "100%" }}
      />
    </div>
  );
}

export default EmailBody;
