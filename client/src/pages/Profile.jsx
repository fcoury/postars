import React from "react";
import Loading from "../components/Loading";
import Sidebar from "../components/Sidebar";
import ToggleSwitch from "../components/ToggleSwitch";
import useProfile from "../hooks/useProfile";
import styles from "./Profile.module.css";

const Profile = () => {
  const { profile, isLoading, error } = useProfile();

  if (isLoading || !profile) {
    return <Loading />;
  }

  const handleEnable = (enabled) => {
    console.log(`Background Indexing ${enabled ? "on" : "off"}`);
  };

  return (
    <>
      <Sidebar />
      <div className={styles.profile}>
        <h2 className={styles.profile__title}>Profile</h2>
        <div className={styles.profile__info}>
          <div className={styles.profile__item}>
            <strong>Name:</strong> <span>{profile.displayName}</span>
          </div>
          <div className={styles.profile__item}>
            <strong>Job Title:</strong> <span>{profile.jobTitle}</span>
          </div>
          <div className={styles.profile__item}>
            <strong>Email:</strong> <span>{profile.mail}</span>
          </div>
          <div className={styles.profile__item}>
            <strong>Preferred Language:</strong>{" "}
            <span>{profile.preferredLanguage}</span>
          </div>
        </div>
        <div className={styles.profile__actions}>
          <label htmlFor="backgroundIndexing">Background Indexing</label>
          <ToggleSwitch id="backgroundIndexing" onChange={handleEnable} />
        </div>
      </div>
    </>
  );
};

export default Profile;
