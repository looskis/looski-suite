"""Parse server-rendered pages as a browser without scripting would submit them."""
from html.parser import HTMLParser
class Forms(HTMLParser):
 """Successful controls per form, as a browser would send them."""
 def __init__(self,html):
  super().__init__();self.forms=[];self.form=None;self.select=None;self.area=None;self.feed(html)
 def handle_starttag(self,tag,attrs):
  attrs=dict(attrs)
  if tag=='form':self.form={'action':attrs.get('action',''),'pairs':[]};self.forms.append(self.form)
  if self.form is None or 'disabled' in attrs:return
  if tag=='input' and 'name' in attrs:
   if attrs.get('type')!='checkbox' or 'checked' in attrs:self.form['pairs'].append((attrs['name'],attrs.get('value','on' if attrs.get('type')=='checkbox' else '')))
  if tag=='textarea' and 'name' in attrs:self.area=[attrs['name'],''];self.form['pairs'].append(self.area)
  if tag=='select':self.select={'name':attrs['name'],'first':None,'chosen':None}
  if tag=='option' and self.select is not None:
   if self.select['first'] is None:self.select['first']=attrs.get('value','')
   if 'selected' in attrs:self.select['chosen']=attrs.get('value','')
 def handle_data(self,data):
  if self.area is not None:self.area[1]+=data
 def handle_endtag(self,tag):
  if tag=='textarea':self.area=None
  if tag=='select' and self.select is not None:
   chosen=self.select['chosen'] if self.select['chosen'] is not None else self.select['first']
   self.form['pairs'].append((self.select['name'],chosen));self.select=None
  if tag=='form':self.form=None
def attribute_values(html):
 """External addresses of the tags that make a browser load or run something."""
 found={'script':[],'img':[],'iframe':[]}
 class Tags(HTMLParser):
  def handle_starttag(self,tag,attrs):
   if tag in found:found[tag].append(dict(attrs).get('src'))
 Tags().feed(html)
 return found
