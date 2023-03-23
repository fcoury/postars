import React from "react";
import { useAppState } from "../../state/AppState";

function EmailBody() {
  const { state } = useAppState();
  const { email } = state;

  if (!email) {
    return <div>Please select an email to view its content.</div>;
  }

  if (email && !email.body) {
    return <div>Loading email body...</div>;
  }

  return (
    <div className="body">
      <div dangerouslySetInnerHTML={{ __html: email.body }} />
    </div>
  );
}

export default EmailBody;
