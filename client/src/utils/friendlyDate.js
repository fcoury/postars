import {
  format,
  formatDistanceToNow,
  isToday,
  isValid,
  isYesterday,
} from "date-fns";

export default function friendlyDate(date, compact = false) {
  // Check if the input date is a valid date object
  if (!isValid(date)) {
    // If not, try to convert it to a date object
    date = new Date(date);
  }

  // If the date is still not valid, return an error message
  if (!isValid(date)) {
    return "Invalid date";
  }

  // Format the date based on the conditions and compact flag
  let formattedDate;
  if (compact) {
    formattedDate = isToday(date)
      ? format(date, "h:mm aa")
      : format(date, "MMM d");
  } else {
    if (isToday(date)) {
      formattedDate = format(date, "h:mm aa");
    } else if (isYesterday(date)) {
      formattedDate = `Yesterday, ${format(date, "h:mm aa")}`;
    } else {
      formattedDate = format(date, "MMM d, h:mm aa");
    }
  }

  // Calculate the relative time, e.g., "15 mins ago"
  const timeAgo = formatDistanceToNow(date, { addSuffix: true });

  return compact || !isToday(date)
    ? formattedDate
    : `${formattedDate} (${timeAgo})`;
}
