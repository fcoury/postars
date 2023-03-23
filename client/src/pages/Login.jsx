import MicrosoftAuthButton from "../components/MicrosoftAuthButton";
import styles from "./Login.module.css";

const Login = () => {
  const config = {
    authority: import.meta.env.VITE_AUTHORITY,
    clientId: import.meta.env.VITE_CLIENT_ID,
    scopes: import.meta.env.VITE_SCOPES.split(" "),
  };

  return (
    <div className={styles.login}>
      <h1 className={styles.title}>Post.rs</h1>
      <MicrosoftAuthButton {...config} />
    </div>
  );
};

export default Login;
