"""Compatibility imports for Parliament declaration response models."""

from exposed.adapters.declaration_models import Category as Category
from exposed.adapters.declaration_models import Declaration as Declaration
from exposed.adapters.declaration_models import DeclarationPage as DeclarationPage
from exposed.adapters.declaration_models import FieldGroup as FieldGroup
from exposed.adapters.declaration_models import FundingEntry as FundingEntry
from exposed.adapters.declaration_models import MemberIdentity as MemberIdentity
from exposed.adapters.declaration_models import Register as Register
from exposed.adapters.declaration_models import Registrant as Registrant
from exposed.adapters.declaration_models import SourceDeclaration as SourceDeclaration
from exposed.adapters.declaration_models import SourceField as SourceField
from exposed.adapters.declaration_models import VersionHeader as VersionHeader
from exposed.core.declarations import canonical_json as canonical_json
from exposed.core.errors import DeclarationParseError as DeclarationParseError
from exposed.core.errors import ParentRequired as ParentRequired
