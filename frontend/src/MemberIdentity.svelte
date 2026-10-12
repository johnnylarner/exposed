<script lang="ts">
  import Icon from "./lib/Icon.svelte";
  import { memberHref, type MemberProfile } from "./lib/entity-details";
  let { member, view }: { member: MemberProfile; view: "summary" | "history" } =
    $props();
</script>

<header class="member-identity">
  <div class="member-identity-title">
    <span class="profile-type"><Icon name="parliament" size={20} />MP</span>
    <h1>{member.name}</h1>
  </div>
  <p>
    {member.partyName || "Party unavailable"}<span
      >{member.membershipFrom || "Membership unavailable"}</span
    ><span>{member.isCurrentCommons ? "Current MP" : "Former MP"}</span>
  </p>
  <nav class="member-tabs" aria-label="Member views">
    <a
      href={memberHref(member.id, "summary")}
      aria-current={view === "summary" ? "page" : undefined}>Summary</a
    >
    <a
      href={memberHref(member.id, "history")}
      aria-current={view === "history" ? "page" : undefined}
      >Declaration history</a
    >
  </nav>
</header>
