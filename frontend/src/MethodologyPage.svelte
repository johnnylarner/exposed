<main id="methodology" tabindex="-1">
  <article aria-labelledby="methodology-heading">
    <header class="article-intro">
      <h1 id="methodology-heading">Methodology</h1>
      <p class="introduction">
        Exposed makes MPs' registered interests easier to search and explore.
        Here is where the data comes from, how we connect records, and what to
        check before using it.
      </p>
    </header>

    <section aria-labelledby="sources">
      <h2 id="sources">Sources and updates</h2>
      <p>
        We use UK Parliament's <a
          href="https://members-api.parliament.uk/index.html">Members API</a
        >
        for member profiles and current House of Commons membership, and its
        <a href="https://interests-api.parliament.uk/index.html"
          >Interests API</a
        > for registered interests. Declarations include expired entries for MPs held
        in our database. This is not a complete history of every former MP.
      </p>
      <p>
        Read the <a
          href="https://www.parliament.uk/mps-lords-and-offices/standards-and-financial-interests/parliamentary-commissioner-for-standards/registers-of-interests/register-of-members-financial-interests/"
          >official Register of Members' Financial Interests</a
        > for the source material.
      </p>
      <p>
        The data is updated daily. Updates reflect what Parliament has
        published; they are not live. A new declaration or correction may take
        time to appear here.
      </p>
    </section>

    <section aria-labelledby="matching">
      <h2 id="matching">Matching and grouping funders</h2>
      <p>
        A funder can appear under several names. We connect records using
        company numbers, names and addresses, and explicitly recorded
        alternative names. Company numbers come from the declarations and are
        not independently verified against Companies House. Conflicting company
        numbers prevent us from treating records as the same legal company.
      </p>
      <p>
        Exact donor names can match even when addresses are missing or
        different. This can join different people or organisations that share a
        name. A similar-looking name alone is not a confirmed match. We preserve
        the original names so you can check the evidence.
      </p>
      <p>
        Separately, we group some funders for browsing using shared recorded
        names, alternative names and selected name families, such as BBC, GB
        News and Premier League. A displayed group is not proof that its entries
        are one legal company or share an owner.
      </p>
      <p>
        A declaration can name a donor, a payer and an ultimate payer. These
        roles can refer to different people or organisations. We use the
        declaration's evidence to attribute each payment once, rather than
        counting it under every named role.
      </p>
    </section>

    <section aria-labelledby="interpretation">
      <h2 id="interpretation">Reading the data</h2>
      <p>
        Totals keep currencies separate. Entries without an amount do not
        contribute to a total; totals affected by missing amounts are marked as
        partial. Party groups use each MP's latest stored party, which may
        differ from their party at the time of a payment.
      </p>
      <p>
        Check the original declarations before publishing research or drawing
        conclusions. Matching and grouping can be wrong, and the source may be
        incomplete or later corrected. A registered interest does not establish
        wrongdoing.
      </p>
    </section>

    <section aria-labelledby="reuse">
      <h2 id="reuse">Licence and citation</h2>
      <p>
        Parliamentary data is available under the <a
          href="https://www.parliament.uk/site-information/copyright-parliament/open-parliament-licence/"
          >Open Parliament Licence v3.0</a
        >. When reusing it, include the required attribution:
      </p>
      <blockquote>
        Contains Parliamentary information licensed under the Open Parliament
        Licence v3.0.
      </blockquote>
      <p>
        Separately, please cite Exposed and link to <a href="/">the project</a> when
        publishing work that uses it. We suggest including the date you accessed the
        data, since declarations and matches can change.
      </p>
    </section>
  </article>
</main>

<style>
  main {
    width: min(100%, 74ch);
  }
  .article-intro {
    margin-bottom: 32px;
  }
  h1 {
    font-size: clamp(36px, 5vw, 54px);
    letter-spacing: -1.8px;
    margin-bottom: 20px;
  }
  p,
  blockquote {
    font-size: 15px;
    line-height: 1.85;
  }
  p {
    margin: 0 0 18px;
  }
  .introduction {
    color: var(--muted);
    font-size: 17px;
    line-height: 1.75;
  }
  a {
    color: var(--accent);
    text-underline-offset: 4px;
  }
  a:hover {
    color: var(--text);
  }
  section {
    margin-top: 44px;
  }
  h2 {
    font-size: 23px;
    font-weight: 600;
    line-height: 1.35;
    letter-spacing: -0.6px;
    margin: 0 0 18px;
  }
  blockquote {
    margin: 24px 0;
    padding-left: 20px;
    border-left: 2px solid var(--accent);
  }
  @media (max-width: 480px) {
    .introduction {
      font-size: 16px;
    }
  }
</style>
