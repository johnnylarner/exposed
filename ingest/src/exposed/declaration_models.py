"""Source declarations and their complete funding projection."""

import json
import re
from decimal import Decimal
from typing import Annotated, Literal, Self

from pydantic import Field, JsonValue

from exposed.models import DisplayName, Model, PositiveID, SourceDate


def canonical_json(value: JsonValue) -> str:
    """Compare source content without conflating JSON booleans and numbers."""
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


class DeclarationPage(Model):
    # Validate each item later so malformed declarations do not discard valid siblings.
    items: list[JsonValue]
    total_results: Annotated[int, Field(ge=0)] = Field(validation_alias="totalResults")
    skip: Annotated[int, Field(ge=0)]


class Category(Model):
    id: PositiveID
    name: DisplayName
    type: Literal["Commons"]


class MemberIdentity(Model):
    id: PositiveID


class Registrant(Model):
    type: Literal["Member"]
    member: MemberIdentity = Field(validation_alias="memberDetail")


class DeclarationParseError(ValueError):
    def __init__(self, path: str, value: object, reason: str):
        super().__init__(f"{path}: {reason}; input={value!r}")


class Register(Model):
    published_date: SourceDate = Field(validation_alias="publishedDate")


class VersionHeader(Model):
    source_register: Register = Field(validation_alias="register")


class SourceField(Model):
    name: str
    type: str | None = None
    type_info: dict[str, JsonValue] | None = Field(default=None, validation_alias="typeInfo")
    value: JsonValue = None
    values: list[list[SourceField]] | None = None

    def check_funding_shape(self, path: str, *, allowed: bool = True, nested: bool = False) -> None:
        has_currency = "currencyCode" in (self.type_info or {})
        financial = self.name in {"Value", "PaymentType", "Donors"} or has_currency
        if financial and (
            not allowed
            or (has_currency and self.name != "Value")
            or (nested and self.name == "Donors")
        ):
            raise DeclarationParseError(
                path, self.model_dump(by_alias=True), "unsupported funding field or nesting"
            )
        for i, group in enumerate(self.values or []):
            if self.name == "Donors" and not group:
                raise DeclarationParseError(f"{path}.values.{i}", [], "empty donor group")
            for j, field in enumerate(group):
                field.check_funding_shape(
                    f"{path}.values.{i}.{j}.{field.name}",
                    allowed=allowed and self.name == "Donors",
                    nested=True,
                )

    def text(self, path: str) -> str | None:
        if self.value is None:
            return None
        if not isinstance(self.value, str):
            raise DeclarationParseError(path, self.value, "expected a string")
        return self.value if self.value.strip() else None


class FundingEntry(Model):
    funder: str | None
    amount: Decimal | None
    currency: str | None
    payment_type: str | None

    @classmethod
    def from_fields(cls, fields: list[SourceField], path: str, *, donor: bool = False) -> Self:
        indexed = {f.name: (i, f) for i, f in enumerate(fields)}
        if len(indexed) != len(fields):
            raise DeclarationParseError(path, [f.name for f in fields], "repeated field names")

        def text(name: str) -> str | None:
            if name not in indexed:
                return None
            i, f = indexed[name]
            return f.text(f"{path}.{i}.{name}.value")

        amount = None
        currency = None
        if "Value" in indexed:
            i, f = indexed["Value"]
            location = f"{path}.{i}.Value"
            if f.type != "Decimal":
                raise DeclarationParseError(location, f.type, "expected Decimal monetary field")
            if f.value is not None:
                if type(f.value) not in (str, int) or not re.fullmatch(
                    r"-?[0-9]+(?:\.[0-9]+)?", str(f.value)
                ):
                    raise DeclarationParseError(location + ".value", f.value, "unsupported amount")
                amount = Decimal(str(f.value))
            code = (f.type_info or {}).get("currencyCode")
            if code is not None and not isinstance(code, str):
                raise DeclarationParseError(
                    location + ".typeInfo.currencyCode", code, "expected a currency string"
                )
            currency = code
        return cls(
            funder=text("UltimatePayerName")
            or text("DonorName")
            or text("PayerName")
            or (text("Name") if donor else None),
            amount=amount,
            currency=currency,
            payment_type=text("PaymentType"),
        )


class FieldGroup(Model):
    fields: list[SourceField]

    def parent_payer_applies(self, path: str) -> bool:
        for i, field in enumerate(self.fields):
            if field.name == "IsUltimatePayerDifferent":
                if field.value is not None and type(field.value) is not bool:
                    raise DeclarationParseError(
                        f"{path}.{i}.IsUltimatePayerDifferent.value",
                        field.value,
                        "expected a boolean",
                    )
                # A withheld different payer must not be attributed to the intermediary.
                return field.value is not True
        return True

    def funding(self, path: str) -> tuple[FundingEntry, ...]:
        entries: list[FundingEntry] = []
        for i, field in enumerate(self.fields):
            field.check_funding_shape(f"{path}.{i}.{field.name}")
        if any(f.name in {"Value", "PaymentType"} for f in self.fields):
            entries.append(FundingEntry.from_fields(self.fields, path))
        for i, field in enumerate(self.fields):
            if field.name == "Donors":
                if field.values is None:
                    raise DeclarationParseError(
                        f"{path}.{i}.Donors.values", None, "expected donor groups"
                    )
                for j, fields in enumerate(field.values):
                    entries.append(
                        FundingEntry.from_fields(
                            fields, f"{path}.{i}.Donors.values.{j}", donor=True
                        )
                    )
        return tuple(entries)


class SourceDeclaration(Model):
    id: PositiveID
    category: Category
    registrant: Registrant
    parent_id: PositiveID | None = Field(default=None, validation_alias="parentInterestId")
    versions: list[dict[str, JsonValue]] = Field(min_length=1)

    def latest_fields(self) -> tuple[FieldGroup, str]:
        dates = [
            VersionHeader.model_validate(v).source_register.published_date for v in self.versions
        ]
        latest = [i for i, date in enumerate(dates) if date == max(dates)]
        first = latest[0]
        contents = [
            {k: v for k, v in self.versions[i].items() if k not in {"register", "links"}}
            for i in latest
        ]
        if any(canonical_json(content) != canonical_json(contents[0]) for content in contents[1:]):
            raise DeclarationParseError(
                "versions", self.versions, "conflicting versions at latest register date"
            )
        return FieldGroup.model_validate(self.versions[first]), f"versions.{first}.fields"


class Declaration(Model):
    id: PositiveID
    category: Category
    registrant: Registrant
    funding: tuple[FundingEntry, ...]
    payer: str | None

    @classmethod
    def from_source(cls, source: SourceDeclaration, *, parent: Self | None = None) -> Self:
        fields, path = source.latest_fields()
        funding = fields.funding(path)
        payer = FundingEntry.from_fields(fields.fields, path).funder
        needs_parent = (
            source.parent_id is not None
            and fields.parent_payer_applies(path)
            and (any(entry.funder is None for entry in funding) or (not funding and payer is None))
        )
        if needs_parent and source.parent_id is not None:
            if parent is None:
                raise ParentRequired(source.parent_id)
            if parent.id != source.parent_id or parent.registrant != source.registrant:
                raise DeclarationParseError(
                    "parentInterestId", source.parent_id, "parent identity or member mismatch"
                )
            if parent.payer is None:
                raise DeclarationParseError(
                    "parentInterestId", source.parent_id, "required parent payer is absent"
                )
            payer = payer or parent.payer
            funding = tuple(
                FundingEntry(
                    funder=entry.funder or parent.payer,
                    amount=entry.amount,
                    currency=entry.currency,
                    payment_type=entry.payment_type,
                )
                for entry in funding
            )
        return cls(
            id=source.id,
            category=source.category,
            registrant=source.registrant,
            funding=funding,
            payer=payer,
        )


class ParentRequired(Exception):
    def __init__(self, source_id: int):
        self.source_id = source_id
